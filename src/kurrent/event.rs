use std::collections::HashMap;

use futures::{stream, Stream, StreamExt};
use kurrentdb::{
    AppendToStreamOptions, EventData, PersistentSubscriptionToAllOptions, ReadStreamOptions,
    StreamPosition, StreamState, SubscriptionFilter,
};
use tracing::instrument;
use uuid::Uuid;

use crate::error::{self, Error};
use crate::event::{Event, EventGroup, Publish, ReplayOne, Sequence, Subscribe};
use crate::kurrent::envelope::KurrentEnvelope;
use crate::kurrent::header::VERSION_KEY;
use crate::kurrent::subject::KurrentSubject;
use crate::kurrent::KurrentStore;
use crate::version::SerializeVersion;

impl Publish for KurrentStore {
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
        let subject = KurrentSubject::Aggregate(E::name().into(), id);
        let options = AppendToStreamOptions::default()
            .stream_state(StreamState::StreamRevision(last_sequence.into()));

        let mut metadata = HashMap::new();
        metadata.insert(VERSION_KEY.to_string(), E::version().to_string());
        let metadata = serde_json::to_string(&metadata).map_err(|e| Error::Format(e.into()))?;

        // FIXME: add type to event or serialization
        let envelope: EventData = EventData::json("Unknown", &event)
            .map_err(|e| Error::Format(e.into()))?
            .id(Uuid::new_v4())
            .metadata(metadata.into());
        let result = self
            .client
            .append_to_stream(subject.into_string(), &options, envelope)
            .await?;
        Ok(Sequence::from(result.next_expected_version))
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
                if let Some(event) = state.next().await.transpose() {
                    let yielded = event;
                    Some((yielded, state))
                } else {
                    None
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
            let stream =
                stream::unfold(
                    sub,
                    |mut state| async move { Some((state.next().await, state)) },
                );
            Ok(stream.map(|m| KurrentEnvelope::try_from_message(m?)))
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
