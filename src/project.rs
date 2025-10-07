use std::ops::Deref;
use std::time::SystemTime;

use uuid::Uuid;

use crate::envelope::{Envelope, TryFromEnvelope};
use crate::event::{self, Sequence};
use crate::{error, EventGroup};

/// A type-safe wrapper for the deserialized contents of an [`Envelope`].
///
/// Once an Event(Group) has been deserialized, placing it inside a Context
/// allows the type to be accessed as normal (with [`Deref`]), while also
/// preventing the Envelope from being deserialized again. A Context also
/// provides accessors for fields like the ID and timestamp of the Envelope.
pub struct Context<'de, E, G> {
    envelope: &'de E,
    event_group: G,
}

/// A data model that can be "projected" onto.
///
/// That is, receive events for all aggregate IDs for an event or events, and
/// process them to trigger a side effect or construct a read model. The exact
/// purpose is implementation specific; the trait only handles receiving the
/// Envelopes for the specified events.
///
/// # Example
/// ```rust
/// # use esrc::project::{Context, Project};
/// # use esrc::version::{DeserializeVersion, SerializeVersion};
/// # use esrc::{Envelope, Event, EventGroup};
/// #
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Event, Deserialize, DeserializeVersion, Serialize, SerializeVersion)]
/// enum FooEvent {
///     Created(usize),
///     Updated(String),
///     Destroyed,
/// }
///
/// # #[derive(Debug, thiserror::Error)]
/// # enum FooError {}
/// #
/// #[derive(Clone)]
/// struct FooProjector {
///     created_sum: usize,
///     last_updated: String,
/// }
///
/// impl<'de> Project<'de> for FooProjector {
///     type EventGroup = FooEvent;
///     type Error = FooError;
///
///     async fn project<E: Envelope>(
///         &mut self,
///         context: Context<'de, E, Self::EventGroup>,
///     ) -> Result<(), Self::Error> {
///         match *context {
///             FooEvent::Created(count) => self.created_sum += count,
///             FooEvent::Updated(ref message) => self.last_updated = message.clone(),
///             FooEvent::Destroyed => { /* trigger a side-effect */ },
///         }
///
///         Ok(())
///     }
/// }
/// ```
#[trait_variant::make(Send)]
pub trait Project: Send + Clone {
    /// The event(s) that can be processed by this object.
    type EventGroup: event::EventGroup + Send + TryFromEnvelope;
    /// The type to return as an `Err` when the projection fails.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Apply a received event, triggering implementation specific behavior.
    ///
    /// Returning an error from this method should stop further messages from
    /// being processed in the associated event store.
    async fn project<'de, E>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: Envelope + Sync;
}

impl<'de, E, G> Context<'de, E, G>
where
    E: Envelope,
    G: EventGroup + TryFromEnvelope,
{
    /// Try to convert the given Envelope into an instance of an EventGroup.
    ///
    /// This uses the [`TryFromEnvelope`] trait to perform the conversion.
    pub fn try_with_envelope(envelope: &'de E) -> error::Result<Self> {
        Ok(Self {
            envelope,
            event_group: G::try_from_envelope(envelope)?,
        })
    }
}

impl<E: Envelope, G> Context<'_, E, G> {
    /// The ID of the referenced Envelope (see [`Envelope::id`]).
    pub fn id(this: &Self) -> Uuid {
        this.envelope.id()
    }

    /// The sequence of the referenced Envelope (see [`Envelope::sequence`]).
    pub fn sequence(this: &Self) -> Sequence {
        this.envelope.sequence()
    }

    /// The timestamp of the referenced Envelope (see [`Envelope::timestamp`]).
    pub fn timestamp(this: &Self) -> SystemTime {
        this.envelope.timestamp()
    }

    /// Get a metadata value from the referenced Envelope (see [`Envelope::get_metadata`]).
    pub fn get_metadata<'a>(this: &'a Self, key: &str) -> Option<&'a str> {
        this.envelope.get_metadata(key)
    }

    /// Unwrap the inner EventGroup instance, and drop the Envelope reference.
    pub fn into_inner(this: Self) -> G {
        this.event_group
    }
}

impl<E, G> Deref for Context<'_, E, G> {
    type Target = G;

    fn deref(&self) -> &Self::Target {
        &self.event_group
    }
}
