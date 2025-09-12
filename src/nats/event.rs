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

        let mut headers = HeaderMap::new();
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

    use futures::TryStreamExt;

    use crate::{
        event::{
            event_model::{Automation, Translation},
            future::IntoSendFuture,
        },
        project::{Context, Project},
    };

    use super::*;

    impl NatsStore {
        #[instrument(skip_all, level = "debug")]
        async fn nats_durable_subscribe<G: EventGroup>(
            &self,
            durable_name: &str,
        ) -> error::Result<impl Stream<Item = error::Result<async_nats::jetstream::Message>> + Send>
        {
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
                .durable_consumer(durable_name.to_string(), subjects)
                .await?;
            Ok(consumer.messages().await?.map_err(|e| e.into()))
        }

        #[instrument(skip_all, level = "debug")]
        async fn run_project<P>(&self, projector: P, durable_name: &str) -> error::Result<()>
        where
            P: Project + 'static,
        {
            let mut stream = pin!(
                self.nats_durable_subscribe::<P::EventGroup>(durable_name)
                    .await?
            );
            while let Some(message) = stream.next().await {
                let message = message?;
                let prefix = self.prefix.to_string();
                let mut projector = projector.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        NatsStore::process_message(&prefix, &mut projector, message).await
                    {
                        tracing::error!("Error processing message: {:?}", e);
                    }
                });
            }

            Ok(())
        }

        #[instrument(skip_all, name = "message", err)]
        async fn process_message<P: Project>(
            prefix: &str,
            projector: &mut P,
            message: async_nats::jetstream::Message,
        ) -> error::Result<()> {
            // attach_span_context(&message);
            let envelope = NatsEnvelope::try_from_message(prefix, message)?;
            let context = Context::try_with_envelope(&envelope)?;
            projector
                .project(context)
                .into_send_future()
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
        async fn start<P>(&self, projector: P, feature_name: &str) -> error::Result<()>
        where
            P: Project + 'static,
        {
            self.run_project(projector, feature_name).await
        }
    }

    impl Translation for NatsStore {
        async fn publish_to_automation<E>(&mut self, id: uuid::Uuid, event: E) -> error::Result<()>
        where
            E: crate::event::Event + crate::version::SerializeVersion,
        {
            let subject = NatsSubject::Event(E::name().into()).into_string(self.prefix);
            let payload = serde_json::to_string(&event).map_err(|e| Error::Format(e.into()))?;

            let mut headers = HeaderMap::new();
            headers.append(VERSION_KEY, E::version().to_string());
            headers.append(EVENT_TYPE, event._type().to_string());
            headers.append("X-Entity-ID", id.to_string());

            let _ack = self
                .context
                .publish_with_headers(subject, headers, payload.into())
                .await?;
            Ok(())
        }
    }
}
