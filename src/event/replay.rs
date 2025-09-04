use std::pin::pin;

use futures::{Stream, StreamExt, TryStreamExt};
use tracing::instrument;
use uuid::Uuid;

use super::future::IntoSendFuture;
use super::{Event, EventGroup, Sequence};
use crate::aggregate::{Aggregate, Root};
use crate::envelope;
use crate::error::{self, Error};
use crate::project::{Context, Project};
use crate::version::DeserializeVersion;

/// Replay all events present in a set of event streams.
#[trait_variant::make(Send)]
pub trait Replay {
    /// The envelope type used by the implementing event store.
    type Envelope: envelope::Envelope;

    /// Replay events starting after the given sequence.
    ///
    /// All events from the streams identified by the EventGroup type parameter
    /// will be included. Processing the resulting Stream will consume these
    /// events in relative order. Any Event published to these streams after
    /// this method is called do not apply; the Stream is finite.
    async fn replay<G: EventGroup>(
        &self,
        first_sequence: Sequence,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send>;
}

/// Replay all events present in a single event stream.
#[trait_variant::make(Send)]
pub trait ReplayOne {
    /// The envelope type used by the implementing event store.
    type Envelope: envelope::Envelope;

    /// Replay events starting after the given sequence.
    ///
    /// All events from the event stream identified by the Event type and the
    /// given Aggregate ID are included. Like [`Replay::replay`], the returned
    /// Stream is finite. This can be used to materialize an Aggregate instace.
    async fn replay_one<E: Event>(
        &self,
        id: Uuid,
        first_sequence: Sequence,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send>;
}

/// Extensions for projecting replayed events.
#[trait_variant::make(Send)]
pub trait ReplayExt: Replay {
    /// Replay all events and project them onto the given Project type.
    ///
    /// Events from all streams identified in the EventGroup type parameter will
    /// be replayed in relative order. Any errors raised by the projection will
    /// cause this method to stop processing any remaining replay-able events.
    async fn rebuild<P>(&self, projector: P) -> error::Result<()>
    where
        P: Project;

    /// Project a subset of events.
    ///
    /// Like [`ReplayExt::rebuild`], this replays any matching events in
    /// relative order, but only with events after the given sequence number.
    async fn rebuild_after<P>(&self, projector: P, first_sequence: Sequence) -> error::Result<()>
    where
        P: Project;
}

/// Extensions for projecting replayed events from a single event stream.
#[trait_variant::make(Send)]
pub trait ReplayOneExt: ReplayOne {
    /// Materialize an aggregate from an event stream.
    ///
    /// Replay all events from the event stream identified by the Aggregate's
    /// Event type and the given Aggregate ID. Each event is applied
    /// to an Aggregate instance to materialize it, with no pre-existing state.
    async fn read<A>(&self, id: Uuid) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: DeserializeVersion;

    /// Update an existing aggregate with new events from a stream.
    ///
    /// Replay events like [`ReplayOneExt::read`], but starting from an existing
    /// aggregate Root. Only events after the last sequence specified in the
    /// Root are replayed. This can be used to read the most recent mutations to
    /// an Aggregate after loading a snapshot.
    async fn read_after<A>(&self, root: Root<A>) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: DeserializeVersion;
}

impl<T> ReplayExt for T
where
    T: Replay + Sync,
    T::Envelope: Sync,
{
    #[instrument(skip_all, level = "debug")]
    async fn rebuild<P>(&self, projector: P) -> error::Result<()>
    where
        P: Project,
    {
        self.rebuild_after(projector, Sequence::new()).await
    }

    #[instrument(skip_all, level = "debug")]
    async fn rebuild_after<P>(
        &self,
        mut projector: P,
        first_sequence: Sequence,
    ) -> error::Result<()>
    where
        P: Project,
    {
        let mut stream = pin!(self.replay::<P::EventGroup>(first_sequence).await?);
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

impl<T> ReplayOneExt for T
where
    T: ReplayOne + Sync,
{
    #[instrument(skip_all, level = "debug")]
    async fn read<A>(&self, id: Uuid) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: DeserializeVersion,
    {
        self.read_after(Root::new(id)).await
    }

    #[instrument(skip_all, level = "debug")]
    async fn read_after<A>(&self, root: Root<A>) -> error::Result<Root<A>>
    where
        A: Aggregate,
        A::Event: DeserializeVersion,
    {
        self.replay_one::<A::Event>(Root::id(&root), Root::last_sequence(&root))
            .await?
            .try_fold(root, |r, e| async move { Root::try_apply(r, e) })
            .await
    }
}
