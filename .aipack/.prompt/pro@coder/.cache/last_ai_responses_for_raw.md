====
>   Info: Duration: 2m 27s | ~$0.1146 | Prompt Tokens: 29,256 | Completion Tokens: 7,807 (reasoning: 2,560) 
>  Model: gpt-5 (gpt-5-2025-08-07) | Adapter: OpenAI
>  Files: Context Files: 44 | Working Files: 0 | Knowledge Files: 0

````rs
// file: src/event/publish.rs
use std::collections::HashMap;
use std::future::Future;

use tracing::instrument;
use uuid::Uuid;

use super::{Event, Sequence};
use crate::aggregate::{Aggregate, Root};
use crate::error::{self, Error};
use crate::version::SerializeVersion;

/// Publish a serializable event to an event stream.
#[trait_variant::make(Send)]
pub trait Publish {
    /// Publish the given Event to an event stream
    ///
    /// The stream is identified by the [`Event::name`] and the given Aggregate
    /// ID. A last sequence is also specified to enforce optimistic concurrency;
    /// if the sequence of the last message in the stream does not match, the
    /// publish will fail. The sequence of the published message is returned.
    async fn publish<E>(
        &mut self,
        id: Uuid,
        last_sequence: Sequence,
        event: E,
        metadata: Option<HashMap<String, String>>,
    ) -> error::Result<Sequence>
    where
        E: Event + SerializeVersion;

    /// Publish the given Event to an event stream without checking the last sequence.
    ///
    /// The stream is identified by the [`Event::name`] and the given Aggregate
    /// ID.
    async fn publish_without_occ<E>(&mut self, id: Uuid, event: E) -> error::Result<()>
    where
        E: Event + SerializeVersion;
}

/// Extensions for publishing events using the aggregate traits.
#[trait_variant::make(Send)]
pub trait PublishExt: Publish {
    /// Apply an Event to an aggregate, after writing it to an event stream.
    ///
    /// The ID and last sequence number are taken from the Root.
    async fn write<A>(
        &mut self,
        root: Root<A>,
        event: A::Event,
        metadata: Option<HashMap<String, String>>,
    ) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: SerializeVersion;

    /// Process a Command, apply the new Event, and write to an event stream.
    ///
    /// Like [`write`], the ID and last sequence number are taken from the Root.
    async fn try_write<A>(
        &mut self,
        root: Root<A>,
        command: A::Command,
        metadata: Option<HashMap<String, String>>,
    ) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: SerializeVersion;
}

impl<T: Publish> PublishExt for T {
    #[instrument(skip_all, level = "debug")]
    async fn write<A>(
        &mut self,
        root: Root<A>,
        event: A::Event,
        metadata: Option<HashMap<String, String>>,
    ) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: SerializeVersion,
    {
        let id = Root::id(&root);
        let last_sequence = Root::last_sequence(&root);

        let aggregate = Root::into_inner(root).apply(&event);
        self.publish::<A::Event>(id, last_sequence, event, metadata)
            .await?;

        Ok(Root::with_aggregate(aggregate, id, last_sequence))
    }

    #[instrument(skip_all, level = "debug")]
    fn try_write<A>(
        &mut self,
        root: Root<A>,
        command: A::Command,
        metadata: Option<HashMap<String, String>>,
    ) -> impl Future<Output = error::Result<Root<A>>>
    where
        A: Aggregate,
        A::Event: SerializeVersion,
    {
        let event = root.process(command).map_err(|e| Error::External(e.into()));
        async move { self.write(root, event?, metadata).await }
    }
}

````

````rs
// file: src/nats/event.rs
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
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> error::Result<Sequence>
    where
        E: Event + SerializeVersion,
    {
        let subject = NatsSubject::Aggregate(E::name().into(), id).into_string(self.prefix);
        let payload = serde_json::to_string(&event).map_err(|e| Error::Format(e.into()))?;

        let mut headers: HeaderMap = {
            if cfg!(feature = "opentelemetry") {
                opentelemetry_nats::NatsHeaderInjector::default_with_span().into()
            } else {
                HeaderMap::new()
            }
        };
        headers.append(VERSION_KEY, E::version().to_string());
        headers.append(
            NATS_EXPECTED_LAST_SUBJECT_SEQUENCE,
            u64::from(last_sequence).to_string(),
        );
        headers.append(EVENT_TYPE, event._type().to_string());

        if let Some(extra) = metadata {
            for (k, v) in extra {
                // avoid overriding reserved keys; NATS headers allow multiple values, but
                // appending duplicate reserved keys may change semantics of retrieval.
                let is_reserved = k.eq_ignore_ascii_case(VERSION_KEY)
                    || k.eq_ignore_ascii_case(EVENT_TYPE)
                    || k.eq_ignore_ascii_case(NATS_EXPECTED_LAST_SUBJECT_SEQUENCE);
                if !is_reserved {
                    headers.append(k, v);
                }
            }
        }

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
                opentelemetry_nats::NatsHeaderInjector::default_with_span().into()
            } else {
                HeaderMap::new()
            }
        };
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

````

````rs
// file: src/kurrent/event.rs
use std::collections::HashMap;

use futures::{stream, Stream, StreamExt};
use kurrentdb::{
    AppendToStreamOptions, EventData, PersistentSubscriptionToAllOptions, ReadStreamOptions,
    StreamPosition, SubscriptionFilter,
};
use tracing::instrument;
use uuid::Uuid;

use crate::error::{self, Error};
use crate::event::{Event, EventGroup, Publish, ReplayOne, Sequence, Subscribe};
use crate::version::SerializeVersion;

use super::envelope::KurrentEnvelope;
use super::header::{EVENT_TYPE, VERSION_KEY};
use super::subject::KurrentSubject;
use super::KurrentStore;

impl Publish for KurrentStore {
    #[instrument(skip_all, level = "debug")]
    async fn publish<E>(
        &mut self,
        id: Uuid,
        // this guarantee is already made by the database
        _last_sequence: Sequence,
        event: E,
        metadata: Option<HashMap<String, String>>,
    ) -> error::Result<Sequence>
    where
        E: Event + SerializeVersion,
    {
        let subject = KurrentSubject::Aggregate(E::name().into(), id);
        let options = AppendToStreamOptions::default();

        let mut metadata = metadata.unwrap_or_default();
        // reserved keys override any provided values
        metadata.insert(VERSION_KEY.to_string(), E::version().to_string());
        metadata.insert(EVENT_TYPE.to_string(), event._type().to_string());
        let metadata = serde_json::to_string(&metadata).map_err(|e| Error::Format(e.into()))?;

        let envelope: EventData = EventData::json(event._type(), &event)
            .map_err(|e| Error::Format(e.into()))?
            .id(Uuid::new_v4())
            .metadata(metadata.into());
        let result = self
            .client
            .append_to_stream(subject.into_string(), &options, envelope)
            .await?;
        Ok(Sequence::from(result.next_expected_version))
    }

    async fn publish_without_occ<E>(&mut self, id: Uuid, event: E) -> error::Result<()>
    where
        E: Event + SerializeVersion,
    {
        let _ = self
            .publish::<E>(id, Sequence::from(0), event, None)
            .await?;
        Ok(())
    }
}

// impl Replay for KurrentStore {
//     type Envelope = KurrentEnvelope;
//
//     #[instrument(skip_all, level = "debug")]
//     async fn replay<G: EventGroup>(
//         &self,
//         first_sequence: Sequence,
//     ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send> {
//         let (_, subjects) = {
//             let mut names = G::names().collect::<Vec<_>>();
//             names.sort();
//
//             let subjects: Vec<_> = names
//                 .iter()
//                 .map(|&n| KurrentSubject::Event(n.into()).into_string())
//                 .collect();
//             (names.join("-"), subjects)
//         };
//
//         let mut filter = SubscriptionFilter::on_stream_name();
//         for subject in subjects {
//             filter = filter.add_prefix(subject);
//         }
//         let options = SubscribeToAllOptions::default().filter(filter);
//
//         let sub = self.client.subscribe_to_all(&options).await;
//         let stream = stream::unfold(sub, |mut state| async move {
//             match state.next().await {
//                 event => Some((event, state)),
//                 _ => None,
//             }
//         });
//         unimplemented!()
//     }
// }

impl ReplayOne for KurrentStore {
    type Envelope = KurrentEnvelope;

    #[instrument(skip_all, level = "debug")]
    async fn replay_one<E: Event>(
        &self,
        id: Uuid,
        first_sequence: Sequence,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send> {
        let subject = KurrentSubject::Aggregate(E::name().into(), id).into_string();
        let options = ReadStreamOptions::default()
            .position(StreamPosition::Position(first_sequence.into()))
            .forwards();
        let stream = stream::unfold(
            self.client.read_stream(subject, &options).await?,
            |mut state| async move {
                match state.next().await {
                    Ok(Some(event)) => Some((Ok(event), state)),
                    Ok(None) => None,
                    Err(kurrentdb::Error::ResourceNotFound) => None,
                    Err(error) => Some((Err(error), state)),
                }
            },
        );
        Ok(stream.map(|m| KurrentEnvelope::try_from_message(m?)))
    }
}

impl Subscribe for KurrentStore {
    type Envelope = KurrentEnvelope;

    #[instrument(skip_all, level = "debug")]
    async fn subscribe<G: EventGroup>(
        &self,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send> {
        let (topic, subjects) = {
            let mut names = G::names().collect::<Vec<_>>();
            names.sort();

            let subjects: Vec<_> = names
                .iter()
                .map(|&n| KurrentSubject::Event(n.into()).into_string())
                .collect();
            (names.join("-"), subjects)
        };

        let mut filter = SubscriptionFilter::on_stream_name();
        for subject in subjects {
            filter = filter.add_prefix(subject);
        }
        let options = PersistentSubscriptionToAllOptions::default().filter(filter);

        match self
            .client
            .create_persistent_subscription_to_all(&topic, &options)
            .await
        {
            Ok(_) => tracing::info!("persistent subscription created"),
            Err(kurrentdb::Error::ResourceAlreadyExists) => {
                tracing::info!("persistent subscription already exists!")
            },
            Err(error) => return Err(error.into()),
        };

        let sub = self
            .client
            .subscribe_to_persistent_subscription_to_all(&topic, &Default::default())
            .await?;
        let stream = stream::unfold(
            sub,
            |mut state| async move { Some((state.next().await, state)) },
        );
        Ok(stream.map(|m| KurrentEnvelope::try_from_message(m?)))
    }
}

// truncate is not supported by the sdk
// impl Truncate for KurrentStore {
//     #[instrument(skip_all, level = "debug")]
//     async fn truncate<E>(&mut self, id: Uuid, last_sequence: Sequence) -> error::Result<()>
//     where
//         E: Event,
//     {
//         unimplemented!()
//     }
// }

pub mod custom {
    use std::pin::pin;

    use crate::{
        event::future::IntoSendFuture,
        project::{Context, Project},
    };

    use super::*;

    impl KurrentStore {
        #[instrument(skip_all, level = "debug")]
        pub async fn durable_subscribe<G: EventGroup>(
            &self,
            durable_name: &str,
        ) -> error::Result<impl Stream<Item = error::Result<KurrentEnvelope>> + Send> {
            let (_, subjects) = {
                let mut names = G::names().collect::<Vec<_>>();
                names.sort();

                let subjects: Vec<_> = names
                    .iter()
                    .map(|&n| KurrentSubject::Event(n.into()).into_string())
                    .collect();
                (names.join("-"), subjects)
            };

            let mut filter = SubscriptionFilter::on_stream_name();
            for subject in subjects {
                filter = filter.add_prefix(subject);
            }
            let options = PersistentSubscriptionToAllOptions::default().filter(filter);

            match self
                .client
                .create_persistent_subscription_to_all(durable_name, &options)
                .await
            {
                Ok(_) => tracing::info!("persistent subscription created"),
                Err(kurrentdb::Error::ResourceAlreadyExists) => {
                    tracing::info!("persistent subscription already exists!")
                },
                Err(error) => return Err(error.into()),
            };

            let sub = self
                .client
                .subscribe_to_persistent_subscription_to_all(durable_name, &Default::default())
                .await?;
            let stream = stream::unfold(sub, |mut state| async move {
                let message = state.next().await;
                match message {
                    Ok(m) => match state.ack(&m).await {
                        Ok(()) => Some((Ok(Ok(m)), state)),
                        Err(error) => Some((Ok(Err(error)), state)),
                    },
                    Err(error) => Some((Err(error), state)),
                }
            })
            .map(|m| {
                let m = m??;
                KurrentEnvelope::try_from_message(m)
            });
            Ok(stream)
        }
    }
    impl KurrentStore {
        #[instrument(skip_all, level = "debug")]
        pub async fn durable_observe<P>(
            &self,
            mut projector: P,
            durable_name: &str,
        ) -> error::Result<()>
        where
            P: Project,
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

````

