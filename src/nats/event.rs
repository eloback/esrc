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

pub mod custom {
    use std::pin::pin;

    use crate::{
        event::future::IntoSendFuture,
        project::{Context, Project},
    };

    use super::*;

    impl NatsStore {
        #[instrument(skip_all, level = "debug")]
        pub async fn durable_subscribe<G: EventGroup>(
            &self,
            durable_name: &str,
        ) -> error::Result<impl Stream<Item = error::Result<NatsEnvelope>> + Send> {
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
            Ok(consumer
                .messages()
                .await?
                .map(|m| NatsEnvelope::try_from_message(self.prefix, m?)))
        }
    }

    impl NatsStore {
        #[instrument(skip_all, level = "debug")]
        pub async fn durable_observe<P>(
            &self,
            mut projector: P,
            durable_name: &str,
        ) -> error::Result<()>
        where
            P: for<'de> Project<'de>,
        {
            let mut stream = pin!(
                self.durable_subscribe::<P::EventGroup>(durable_name)
                    .await?
            );
            while let Some(envelope) = stream.next().await {
                let envelope = envelope?;
                let context = Context::try_with_envelope(&envelope)?;

                projector
                    .project(context)
                    .into_send_future()
                    .await
                    .map_err(|e| Error::External(e.into()))?;
            }

            Ok(())
        }
    }
}
