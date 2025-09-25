use async_nats::header::NATS_EXPECTED_LAST_SUBJECT_SEQUENCE;
use async_nats::HeaderMap;
use futures::{Stream, StreamExt};
use tracing::instrument;
use uuid::Uuid;

use super::header::{EVENT_TYPE, VERSION_KEY};
use super::subject::NatsSubject;
use super::{NatsEnvelope, NatsStore};
use crate::error::{self, Error};
use crate::event::{Event, EventGroup, Publish, Replay, ReplayOne, Sequence, Subscribe, Truncate};
use crate::version::SerializeVersion;

impl Publish for NatsStore {
    #[instrument(skip_all, level = "debug")]
    async fn publish<E>(
        &mut self,
        id: Uuid,
        last_sequence: Sequence,
        event: E,
    ) -> error::Result<Sequence>
    where
        E: Event + SerializeVersion,
    {
        let subject = NatsSubject::Aggregate(E::name().into(), id).into_string(self.prefix);
        let payload = serde_json::to_string(&event).map_err(|e| Error::Format(e.into()))?;

        let mut headers: HeaderMap = {
            if cfg!(feature = "opentelemetry") {
                // create with openslemetry headers
                opentelemetry_nats::NatsHeaderInjector::default_with_span().into()
            } else {
                HeaderMap::new()
            }
        };
        // add esrc headers
        headers.append(VERSION_KEY, E::version().to_string());
        headers.append(
            NATS_EXPECTED_LAST_SUBJECT_SEQUENCE,
            u64::from(last_sequence).to_string(),
        );
        headers.append(EVENT_TYPE, event._type().to_string());

        let ack = self
            .context
            .publish_with_headers(subject, headers, payload.into())
            .await?;
        Ok(Sequence::from(ack.await?.sequence))
    }

    async fn publish_without_occ<E>(&mut self, id: Uuid, event: E) -> error::Result<()>
    where
        E: Event + SerializeVersion,
    {
        let subject = NatsSubject::Aggregate(E::name().into(), id).into_string(self.prefix);
        let payload = serde_json::to_string(&event).map_err(|e| Error::Format(e.into()))?;

        let mut headers: HeaderMap = {
            if cfg!(feature = "opentelemetry") {
                // create with openslemetry headers
                opentelemetry_nats::NatsHeaderInjector::default_with_span().into()
            } else {
                HeaderMap::new()
            }
        };
        // add esrc headers
        headers.append(VERSION_KEY, E::version().to_string());
        headers.append(EVENT_TYPE, event._type().to_string());

        let _ = self
            .context
            .publish_with_headers(subject, headers, payload.into())
            .await?
            .await?;
        Ok(())
    }
}

impl Replay for NatsStore {
    type Envelope = NatsEnvelope;

    #[instrument(skip_all, level = "debug")]
    async fn replay<G: EventGroup>(
        &self,
        first_sequence: Sequence,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send> {
        let subjects = G::names()
            .map(|n| NatsSubject::Event(n.into()).into_string(self.prefix))
            .collect();
        let consumer = self
            .ordered_consumer(subjects, first_sequence.into())
            .await?;

        let pending = consumer.cached_info().num_pending as usize;
        Ok(consumer
            .messages()
            .await?
            .take(pending)
            .map(|m| NatsEnvelope::try_from_message(self.prefix, m?)))
    }
}

impl ReplayOne for NatsStore {
    type Envelope = NatsEnvelope;

    #[instrument(skip_all, level = "debug")]
    async fn replay_one<E: Event>(
        &self,
        id: Uuid,
        first_sequence: Sequence,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send> {
        let subject = NatsSubject::Aggregate(E::name().into(), id).into_string(self.prefix);
        let consumer = self
            .ordered_consumer(vec![subject], first_sequence.into())
            .await?;

        let pending = consumer.cached_info().num_pending as usize;
        Ok(consumer
            .messages()
            .await?
            .take(pending)
            .map(|m| NatsEnvelope::try_from_message(self.prefix, m?)))
    }
}

impl Subscribe for NatsStore {
    type Envelope = NatsEnvelope;

    #[instrument(skip_all, level = "debug")]
    async fn subscribe<G: EventGroup>(
        &self,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send> {
        let (topic, subjects) = {
            let mut names = G::names().collect::<Vec<_>>();
            names.sort();

            let subjects = names
                .iter()
                .map(|&n| NatsSubject::Event(n.into()).into_string(self.prefix))
                .collect();
            (names.join("-"), subjects)
        };

        let consumer = self.durable_consumer(topic, subjects).await?;
        Ok(consumer
            .messages()
            .await?
            .map(|m| NatsEnvelope::try_from_message(self.prefix, m?)))
    }
}

impl Truncate for NatsStore {
    #[instrument(skip_all, level = "debug")]
    async fn truncate<E>(&mut self, id: Uuid, last_sequence: Sequence) -> error::Result<()>
    where
        E: Event,
    {
        let subject = NatsSubject::Aggregate(E::name().into(), id).into_string(self.prefix);
        self.context
            .get_stream(&self.prefix)
            .await?
            .purge()
            .filter(subject)
            .sequence(last_sequence.into())
            .await?;

        Ok(())
    }
}

pub mod event_model {
    use std::pin::pin;

    use stream_cancel::Valved;

    use crate::{
        event::event_model::{Automation, Translation, ViewAutomation},
        project::{Context, Project},
        Envelope,
    };

    use super::*;

    impl NatsStore {
        /// recieves a message, processes it with the given projector, and acknowledges it.
        #[instrument(skip_all, name = "automation", level = "info", fields(aggregate=tracing::field::Empty) err)]
        async fn process_message<P: Project>(
            projector: &mut P,
            message: Result<NatsEnvelope, Error>,
        ) -> error::Result<()> {
            let envelope = message?;
            envelope.attach_span_context();
            tracing::Span::current().record("aggregate", envelope.name());
            let context = Context::try_with_envelope(&envelope)?;
            projector
                .project(context)
                .await
                .map_err(|e| Error::External(e.into()))?;
            envelope.ack().await;
            Ok(())
        }
    }

    impl Automation for NatsStore {
        type Envelope = NatsEnvelope;

        #[instrument(skip_all, level = "debug")]
        async fn durable_subscribe<G: EventGroup>(
            &self,
            unique_name: &str,
        ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send> {
            let (_, subjects) = {
                let mut names = G::names().collect::<Vec<_>>();
                names.sort();

                let subjects = names
                    .iter()
                    .map(|&n| NatsSubject::Event(n.into()).into_string(self.prefix))
                    .collect();
                (names.join("-"), subjects)
            };

            let consumer = self
                .durable_consumer(unique_name.to_string(), subjects)
                .await?;
            Ok(consumer
                .messages()
                .await?
                .map(|m| NatsEnvelope::try_from_message(self.prefix, m?)))
        }

        #[instrument(skip_all, level = "debug")]
        async fn start_automation<P>(&self, projector: P, feature_name: &str) -> error::Result<()>
        where
            P: Project + 'static,
        {
            let stream = pin!(
                self.durable_subscribe::<P::EventGroup>(feature_name)
                    .await?,
            );

            let (exit, incoming) = Valved::new(stream);
            self.graceful_shutdown
                .exit_tx
                .send(exit)
                .await
                .expect("should be able to send graceful trigger");

            let mut incoming = incoming;

            while let Some(message) = incoming.next().await {
                let mut projector = projector.clone();

                self.graceful_shutdown.task_tracker.spawn(async move {
                    if let Err(e) = NatsStore::process_message(&mut projector, message).await {
                        tracing::error!("Error processing message: {:?}", e);
                    }
                });
            }

            Ok(())
        }
    }

    impl Translation for NatsStore {
        async fn publish_to_automation<E>(&mut self, id: uuid::Uuid, event: E) -> error::Result<()>
        where
            E: crate::event::Event + crate::version::SerializeVersion,
        {
            let subject = NatsSubject::Aggregate(E::name().into(), id).into_string(self.prefix);
            let payload = serde_json::to_string(&event).map_err(|e| Error::Format(e.into()))?;

            let mut headers: HeaderMap = {
                if cfg!(feature = "opentelemetry") {
                    // create with openslemetry headers
                    opentelemetry_nats::NatsHeaderInjector::default_with_span().into()
                } else {
                    HeaderMap::new()
                }
            };
            headers.append(VERSION_KEY, E::version().to_string());
            headers.append(EVENT_TYPE, event._type().to_string());

            let _ack = self
                .context
                .publish_with_headers(subject, headers, payload.into())
                .await?;
            Ok(())
        }
    }

    impl ViewAutomation for NatsStore {
        #[instrument(skip_all, level = "debug")]
        async fn start_view_automation<P>(
            &self,
            projector: P,
            feature_name: &str,
        ) -> error::Result<()>
        where
            P: Project + 'static,
        {
            let stream = pin!(
                self.durable_subscribe::<P::EventGroup>(feature_name)
                    .await?
            );
            let (exit, mut incoming) = Valved::new(stream);
            self.graceful_shutdown
                .exit_tx
                .clone()
                .send(exit)
                .await
                .expect("should be able to send graceful trigger");

            while let Some(message) = incoming.next().await {
                let mut projector = projector.clone();

                if let Err(e) = NatsStore::process_message(&mut projector, message).await {
                    tracing::error!("Error processing message: {:?}", e);
                }
            }

            Ok(())
        }
    }
}
