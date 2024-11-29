use std::time::SystemTime;

use uuid::Uuid;

use crate::error;
use crate::event::{Event, Sequence};
use crate::version::DeserializeVersion;

#[cfg(feature = "derive")]
#[doc(inline)]
pub use esrc_derive::TryFromEnvelope;

/// Data associated with a single event in a stream.
///
/// This data is held in a generic structure that can be deserialized into a
/// specific event type. Only used when reading from an event store.
///
/// _This trait should only need to be implemented within event store backends;
/// it represents event data loaded directly from the store_.
pub trait Envelope: Send {
    /// The ID of the event stream this envelope belongs to.
    ///
    /// This is equivalent to the ID of the
    /// [Aggregate](`crate::aggregate::Aggregate`) that this event applies to.
    fn id(&self) -> Uuid;
    /// The position of this event in its event stream.
    fn sequence(&self) -> Sequence;

    /// The approximate time that the event was originally published.
    fn timestamp(&self) -> SystemTime;

    /// The name of the event stream this event belongs to.
    ///
    /// This name will correspond to the [`Event::name`] of the event type that
    /// can be deserialized from this envelope.
    fn name(&self) -> &str;
    /// Parse the inner [`Event`] type from this envelope.
    ///
    /// The exact format that is used and deserialized is dependent on the event
    /// store backend. An error will be returned if the given type of event is
    /// not stored in this envelope, or there is an error with the data format.
    fn deserialize<'de, E>(&'de self) -> error::Result<E>
    where
        E: DeserializeVersion<'de> + Event;
}

/// Attempt to deserialize an Envelope into the implementing type.
///
/// This trait is automatically implemented for all [`Event`] types. It can also
/// be implemented manually (or with the derive macro) for union types holding
/// multiple events, as an [EventGroup](`crate::event::EventGroup`).
///
/// # Example
/// ```rust
/// # use esrc::envelope::TryFromEnvelope;
/// # use esrc::version::{DeserializeVersion, SerializeVersion};
/// # use esrc::{Envelope, Error, Event};
/// #
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Event, Deserialize, DeserializeVersion, Serialize, SerializeVersion)]
/// struct FooEvent;
///
/// #[derive(Event, Deserialize, DeserializeVersion, Serialize, SerializeVersion)]
/// struct BarEvent;
///
/// enum BazEvents {
///     Foo(FooEvent),
///     Bar(BarEvent),
/// }
///
/// impl<'de> TryFromEnvelope<'de> for BazEvents {
///     fn try_from_envelope(envelope: &'de impl Envelope) -> Result<Self, Error> {
///         if envelope.name() == FooEvent::name() {
///             Ok(BazEvents::Foo(envelope.deserialize()?))
///         } else {
///             Ok(BazEvents::Bar(envelope.deserialize()?))
///         }
///     }
/// }
/// ```
pub trait TryFromEnvelope<'de>: Sized {
    /// Attempt to deserialize the content of an Envelope and convert it.
    ///
    /// This method takes a reference to the Envelope, as the implementor may
    /// have its own lifetimes that will be dependent of the deserializer
    /// lifetime of the Envelope (for zero-copy deserialization, etc).
    fn try_from_envelope(envelope: &'de impl Envelope) -> error::Result<Self>;
}

impl<'de, E> TryFromEnvelope<'de> for E
where
    E: DeserializeVersion<'de> + Event,
{
    fn try_from_envelope(envelope: &'de impl Envelope) -> error::Result<Self> {
        envelope.deserialize()
    }
}
