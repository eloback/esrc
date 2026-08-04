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
    async fn publish_without_occ<E>(
        &mut self,
        id: Uuid,
        event: E,
        metadata: Option<HashMap<String, String>>,
    ) -> error::Result<()>
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
        let published_sequence = self
            .publish::<A::Event>(id, last_sequence, event, metadata)
            .await?;

        Ok(Root::with_aggregate(aggregate, id, published_sequence))
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

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::*;

    #[derive(Serialize)]
    struct Added(u64);

    impl Event for Added {
        fn name() -> &'static str {
            "Added"
        }
    }

    impl SerializeVersion for Added {
        fn version() -> usize {
            1
        }
    }

    #[derive(Default)]
    struct Counter(u64);

    enum CounterCommand {}

    #[derive(Debug, thiserror::Error)]
    #[error("counter command failed")]
    struct CounterError;

    impl Aggregate for Counter {
        type Command = CounterCommand;
        type Event = Added;
        type Error = CounterError;

        fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
            match command {}
        }

        fn apply(mut self, event: &Self::Event) -> Self {
            self.0 += event.0;
            self
        }
    }

    struct MockPublish {
        acknowledged_sequence: Sequence,
    }

    impl Publish for MockPublish {
        async fn publish<E>(
            &mut self,
            _id: Uuid,
            _last_sequence: Sequence,
            _event: E,
            _metadata: Option<HashMap<String, String>>,
        ) -> error::Result<Sequence>
        where
            E: Event + SerializeVersion,
        {
            Ok(self.acknowledged_sequence)
        }

        async fn publish_without_occ<E>(
            &mut self,
            _id: Uuid,
            _event: E,
            _metadata: Option<HashMap<String, String>>,
        ) -> error::Result<()>
        where
            E: Event + SerializeVersion,
        {
            unreachable!("the write extension uses OCC publication")
        }
    }

    #[tokio::test]
    async fn write_returns_the_acknowledged_sequence() {
        let expected_sequence = Sequence::from(42);
        let mut publisher = MockPublish {
            acknowledged_sequence: expected_sequence,
        };
        let root = Root::<Counter>::new(Uuid::nil());

        let written = publisher.write(root, Added(7), None).await.unwrap();

        assert_eq!(u64::from(Root::last_sequence(&written)), 42);
        assert_eq!(written.0, 7);
    }
}
