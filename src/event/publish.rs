use std::future::Future;

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
    ) -> error::Result<Sequence>
    where
        E: Event + SerializeVersion;
}

/// Extensions for publishing events using the aggregate traits.
#[trait_variant::make(Send)]
pub trait PublishExt: Publish {
    /// Apply an Event to an aggregate, after writing it to an event stream.
    ///
    /// The ID and last sequence number are taken from the Root.
    async fn write<A>(&mut self, root: Root<A>, event: A::Event) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: SerializeVersion;

    /// Process a Command, apply the new Event, and write to an event stream.
    ///
    /// Like [`write`], the ID and last sequence number are taken from the Root.
    async fn try_write<A>(&mut self, root: Root<A>, command: A::Command) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: SerializeVersion;
}

impl<T: Publish> PublishExt for T {
    async fn write<A>(&mut self, root: Root<A>, event: A::Event) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: SerializeVersion,
    {
        let id = Root::id(&root);
        let last_sequence = Root::last_sequence(&root);

        let aggregate = Root::into_inner(root).apply(&event);
        self.publish::<A::Event>(id, last_sequence, event).await?;

        Ok(Root::with_aggregate(aggregate, id, last_sequence))
    }

    fn try_write<A>(
        &mut self,
        root: Root<A>,
        command: A::Command,
    ) -> impl Future<Output = error::Result<Root<A>>>
    where
        A: Aggregate,
        A::Event: SerializeVersion,
    {
        let event = root.process(command).map_err(|e| Error::External(e.into()));
        async move { self.write(root, event?).await }
    }
}
