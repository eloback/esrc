use uuid::Uuid;

use super::{Event, Sequence};
use crate::error;

/// Truncate/delete old messages from an event stream.
#[trait_variant::make(Send)]
pub trait Truncate {
    /// Truncate all messages after the specified sequence.
    ///
    /// Any events in the stream specified by the Event type and given aggregate
    /// ID will be dropped from the stream, if they have a sequence number
    /// lesser than the passed value. This can be used along with snapshotting
    /// to manage the size of event streams.
    async fn truncate<E>(&mut self, id: Uuid, last_sequence: Sequence) -> error::Result<()>
    where
        E: Event;
}
