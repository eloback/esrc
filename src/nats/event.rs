use async_nats::header::NATS_EXPECTED_LAST_SUBJECT_SEQUENCE;
use async_nats::jetstream::stream::{LastRawMessageErrorKind, Stream as JetStream};
use futures::{Stream, StreamExt};
use tracing::instrument;
use uuid::Uuid;

use super::header::{self, EVENT_TYPE, METADATA_PREFIX, VERSION_KEY};
use super::subject::NatsSubject;
use super::{effective_ack_wait, NatsEnvelope, NatsStore};
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
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> error::Result<Sequence>
    where
        E: Event + SerializeVersion,
    {
        let subject = NatsSubject::Aggregate(E::name().into(), id).into_string(self.prefix);
        let payload = serde_json::to_string(&event).map_err(|e| Error::Format(e.into()))?;
        let last_sequence = u64::from(last_sequence);
        let mut headers = header::new();
        headers.append(VERSION_KEY, E::version().to_string());
        headers.append(
            NATS_EXPECTED_LAST_SUBJECT_SEQUENCE,
            last_sequence.to_string(),
        );
        headers.append(EVENT_TYPE, event._type().to_string());

        if let Some(extra) = metadata {
            for (k, v) in extra {
                // avoid overriding reserved keys;
                let k = format!("{METADATA_PREFIX}{k}");
                headers.append(k, v);
            }
        }

        let ack = self
            .context
            .publish_with_headers(subject, headers, payload.into())
            .await?;
        let ack = ack.await?;
        Ok(Sequence::from(ack.sequence))
    }

    async fn publish_without_occ<E>(
        &mut self,
        id: Uuid,
        event: E,
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> error::Result<()>
    where
        E: Event + SerializeVersion,
    {
        let subject = NatsSubject::Aggregate(E::name().into(), id).into_string(self.prefix);
        let payload = serde_json::to_string(&event).map_err(|e| Error::Format(e.into()))?;

        let mut headers = header::new();
        headers.append(VERSION_KEY, E::version().to_string());
        headers.append(EVENT_TYPE, event._type().to_string());

        if let Some(extra) = metadata {
            for (k, v) in extra {
                // avoid overriding reserved keys; NATS headers allow multiple values, but
                // appending duplicate reserved keys may change semantics of retrieval.
                let k = format!("{METADATA_PREFIX}{k}");
                headers.append(k, v);
            }
        }

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
        let start_sequence = u64::from(first_sequence)
            .checked_add(1)
            .ok_or(Error::Invalid)?;
        let consumer = self.ordered_consumer(subjects, start_sequence).await?;

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
        let start_sequence = u64::from(first_sequence)
            .checked_add(1)
            .ok_or(Error::Invalid)?;
        let stream = self.reader_stream().clone();
        let last_sequence = match stream.get_last_raw_message_by_subject(&subject).await {
            Ok(message) if message.sequence >= start_sequence => Some(message.sequence),
            Ok(_) => None,
            Err(error) if error.kind() == LastRawMessageErrorKind::NoMessageFound => None,
            Err(error) => return Err(Error::Internal(error.into())),
        };

        Ok(futures::stream::try_unfold(
            RawReplayState {
                stream,
                subject,
                next_sequence: last_sequence.map(|_| start_sequence),
                last_sequence,
                prefix: self.prefix,
            },
            next_raw_replay_message,
        ))
    }
}

struct RawReplayState {
    stream: JetStream,
    subject: String,
    next_sequence: Option<u64>,
    last_sequence: Option<u64>,
    prefix: &'static str,
}

async fn next_raw_replay_message(
    mut state: RawReplayState,
) -> error::Result<Option<(NatsEnvelope, RawReplayState)>> {
    let (Some(next_sequence), Some(last_sequence)) = (state.next_sequence, state.last_sequence)
    else {
        return Ok(None);
    };

    let message = state
        .stream
        .get_first_raw_message_by_subject(&state.subject, next_sequence)
        .await
        .map_err(|error| Error::Internal(error.into()))?;
    if message.sequence < next_sequence || message.sequence > last_sequence {
        return Err(Error::Internal(
            std::io::Error::other(format!(
                "aggregate replay returned stream sequence {} outside the captured range {next_sequence}..={last_sequence}",
                message.sequence
            ))
            .into(),
        ));
    }

    state.next_sequence = if message.sequence == last_sequence {
        None
    } else {
        Some(message.sequence.checked_add(1).ok_or(Error::Invalid)?)
    };
    let envelope = NatsEnvelope::try_from_stream_message(state.prefix, message)?;
    Ok(Some((envelope, state)))
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
        event::event_model::{
            Automation, Translation, ViewAutomation, ViewProjectorIdentity,
            DEFAULT_VIEW_PROJECTOR_VERSION,
        },
        project::{Context, Project},
        Envelope,
    };

    use super::*;

    impl NatsStore {
        /// recieves a message, processes it with the given projector, and acknowledges it.
        #[instrument(skip_all, name = "automation", level = "info", fields(aggregate=tracing::field::Empty) err(Debug))]
        async fn process_message<P: Project>(
            projector: &P,
            message: Result<NatsEnvelope, Error>,
        ) -> error::Result<()> {
            let envelope = message?;
            envelope.attach_span_context();
            tracing::Span::current().record("aggregate", envelope.name());
            let context = Context::try_with_envelope(&envelope)?;
            let mut projector = projector.clone();
            projector
                .project(context)
                .await
                .map_err(|e| Error::External(e.into()))?;
            envelope.ack().await?;
            Ok(())
        }

        /// Process one live-view delivery while renewing its acknowledgement lease.
        async fn process_view_message<P: Project>(
            projector: &mut P,
            message: Result<NatsEnvelope, Error>,
            ack_wait: std::time::Duration,
        ) -> error::Result<()> {
            let envelope = message?;
            envelope.attach_span_context();
            tracing::Span::current().record("aggregate", envelope.name());
            {
                let context = Context::try_with_envelope(&envelope)?;
                let projection = projector.project(context);
                tokio::pin!(projection);

                let progress_interval =
                    std::cmp::max(ack_wait / 3, std::time::Duration::from_millis(1));
                let mut progress = tokio::time::interval(progress_interval);
                progress.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                progress.tick().await;

                loop {
                    tokio::select! {
                        result = &mut projection => {
                            result.map_err(|error| Error::External(error.into()))?;
                            break;
                        }
                        _ = progress.tick() => envelope.in_progress().await?,
                    }
                }
            }

            envelope.ack().await?;
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
        async fn start_automation<P>(
            &self,
            projector: P,
            feature_name: &str,
            max_concurrency: impl Into<Option<usize>> + Send,
        ) -> error::Result<()>
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

            // Configure throughput (concurrent workers)
            incoming
                .for_each_concurrent(max_concurrency, |message| {
                    let projector = projector.clone();

                    async move {
                        if let Err(e) = NatsStore::process_message(&projector, message).await {
                            tracing::error!("Error processing message: {:?}", e);
                        }
                    }
                })
                .await;

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

            let mut headers = header::new();
            headers.append(VERSION_KEY, E::version().to_string());
            headers.append(EVENT_TYPE, event._type().to_string());

            self.context
                .publish_with_headers(subject, headers, payload.into())
                .await?
                .await?;
            Ok(())
        }
    }

    impl ViewAutomation for NatsStore {
        async fn start_view_automation<P>(
            &self,
            projector: P,
            feature_name: &str,
        ) -> error::Result<()>
        where
            P: Project + 'static,
        {
            self.start_view_automation_with_identity(
                projector,
                feature_name,
                ViewProjectorIdentity::new(feature_name, DEFAULT_VIEW_PROJECTOR_VERSION),
            )
            .await
        }

        #[instrument(skip_all, level = "debug")]
        async fn start_view_automation_with_identity<P>(
            &self,
            projector: P,
            feature_name: &str,
            identity: ViewProjectorIdentity,
        ) -> error::Result<()>
        where
            P: Project + 'static,
        {
            let subjects = {
                let mut names = P::EventGroup::names().collect::<Vec<_>>();
                names.sort();
                names
                    .into_iter()
                    .map(|name| NatsSubject::Event(name.into()).into_string(self.prefix))
                    .collect()
            };
            let consumer = self
                .view_durable_consumer(feature_name.to_owned(), subjects, &identity)
                .await?;
            let consumer_config = &consumer.cached_info().config;
            let ack_wait = effective_ack_wait(consumer_config.ack_wait, &consumer_config.backoff);
            let stream = pin!(consumer
                .messages()
                .await?
                .map(|message| NatsEnvelope::try_from_message(self.prefix, message?)));
            let (exit, mut incoming) = Valved::new(stream);
            self.graceful_shutdown
                .exit_tx
                .clone()
                .send(exit)
                .await
                .expect("should be able to send graceful trigger");

            let mut projector = projector;
            while let Some(message) = incoming.next().await {
                if let Err(e) =
                    NatsStore::process_view_message(&mut projector, message, ack_wait).await
                {
                    tracing::error!("Error processing message: {:?}", e);
                }
            }

            Ok(())
        }
    }
}
