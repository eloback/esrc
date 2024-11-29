use std::iter;

/// Publish events to an event store.
pub mod publish;
/// Replay existing events in an event store.
pub mod replay;
/// Subscribe to new events from an event store.
pub mod subscribe;
/// Truncate (delete) old events from an event store.
pub mod truncate;

pub use publish::{Publish, PublishExt};
pub use replay::{Replay, ReplayExt, ReplayOne, ReplayOneExt};
pub use subscribe::{Subscribe, SubscribeExt};
pub use truncate::Truncate;

#[cfg(feature = "derive")]
#[doc(inline)]
pub use esrc_derive::{Event, EventGroup};

mod future;

/// The relative order of an Event within a single event stream.
///
/// An Event published to a stream will always have a greater sequence number
/// than a previously published event. Tracking the last sequence in a stream is
/// important for implementing optimistic concurrency when publishing events.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Sequence(u64);

/// Data representing a state change in the event sourced system.
///
/// Each event type should hold all actions / "events" that can be applied to
/// the corresponding Aggregate (eg, within an enum). Most of these properties
/// are implementation specific and not constrained by the library.
pub trait Event: Send {
    /// A unique identifier to represent event streams of this type.
    /// The characters allowed for a name are backend specific, and are best
    /// kept to ASCII letters and numbers. Once used, the name of an event
    /// should stay the same, even if the data structure changes.
    fn name() -> &'static str;
}

/// Reference multiple event types in a single read stream.
///
/// This is used to listen/replay messages for multiple aggregates from a single
/// spot when projecting to a read model (rather than requiring multiple streams
/// and readers for the same functionality).
///
/// Note that this trait is implemented automatically for all [events](`Event`),
/// such that they are treated as a single-event group.
pub trait EventGroup {
    /// The names of all events within this Group.
    fn names() -> impl Iterator<Item = &'static str>;
}

impl Sequence {
    /// Create a starting sequence that is always valid in a new event stream.
    pub fn new() -> Self {
        Sequence(0)
    }
}

impl<E: Event> EventGroup for E {
    fn names() -> impl Iterator<Item = &'static str> {
        iter::once(E::name())
    }
}

impl From<Sequence> for u64 {
    fn from(value: Sequence) -> Self {
        value.0
    }
}

impl From<u64> for Sequence {
    fn from(value: u64) -> Self {
        Sequence(value)
    }
}
