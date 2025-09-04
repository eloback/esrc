use std::ops::Deref;

use uuid::Uuid;

use crate::envelope::Envelope;
use crate::error::{self, Error};
use crate::event::{self, Event, Sequence};
use crate::version::DeserializeVersion;

/// Data that can be represented/constructed from an Event stream.
///
/// This trait and its methods refer to the Domain Driven Design aggregate,
/// which is a grouping of domain objects that represent a transactional
/// boundary and a single unit. An aggregate is modified by processing Commands,
/// which emit [events](`Event`) that will be written to an event stream.
///
/// # Example
/// ```rust
/// # use esrc::{Aggregate, Event};
/// #
/// #[derive(Default)]
/// struct PowerSwitch(bool);
///
/// enum PowerCommand {
///     Toggle,
/// }
///
/// #[derive(Event)]
/// enum PowerEvent {
///     TurnedOn,
///     TurnedOff,
/// }
///
/// # #[derive(Debug, thiserror::Error)]
/// # enum PowerError {}
/// #
/// impl Aggregate for PowerSwitch {
///     type Command = PowerCommand;
///     type Event = PowerEvent;
///     type Error = PowerError; // A type implementing `std::error::Error`.
///
///     fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
///         if self.0 {
///             Ok(PowerEvent::TurnedOff)
///         } else {
///             Ok(PowerEvent::TurnedOn)
///         }
///     }
///
///     fn apply(self, event: &Self::Event) -> Self {
///         match event {
///             PowerEvent::TurnedOff => PowerSwitch(false),
///             PowerEvent::TurnedOn => PowerSwitch(true),
///         }
///     }
/// }
/// ```
pub trait Aggregate: Default + Send {
    /// The type used to alter the state of an aggregate.
    ///
    /// This will likely be an enum type, analogous to the Event type, that can
    /// hold different actions for the aggregate to process.
    type Command;
    /// The event type to emit while processing commands.
    ///
    /// An aggregate can be constructed from this type by applying a sequence of
    /// events. The name of the event, along with the ID of the aggregate, is
    /// also used to uniquely identify an aggregate instance.
    type Event: event::Event;
    /// The type to return as an `Err` when processing a command fails.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Evaluate a command, resulting in a single event or an error.
    ///
    /// No aggregate state should be changed in this method; the change is
    /// applied once the event is published to the event store.
    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error>;
    /// Update the aggregate state using a previously published event.
    fn apply(self, event: &Self::Event) -> Self;
}

/// A materialized aggregate at a specific point in an event stream.
///
/// This holds an [`Aggregate`] instance alongside information
/// needed to publish events for that instance, such as the aggregate ID.
pub struct Root<A> {
    id: Uuid,
    last_sequence: Sequence,

    aggregate: A,
}

impl<A> Root<A> {
    /// Create a new Root from an existing aggregate.
    ///
    /// This requires the ID and sequence number to be manually specified.
    pub fn with_aggregate(aggregate: A, id: Uuid, last_sequence: Sequence) -> Self {
        Self {
            id,
            last_sequence,

            aggregate,
        }
    }

    /// The ID of this aggregate.
    ///
    /// This is used to associate a specific event stream with this instance.
    pub fn id(this: &Self) -> Uuid {
        this.id
    }

    /// The sequence number of the last published event for this aggregate.
    ///
    /// This is used to enable optimistic concurrency when publishing events.
    pub fn last_sequence(this: &Self) -> Sequence {
        this.last_sequence
    }

    /// The actual aggregate instance from this Root.
    ///
    /// Converting to the inner type loses the identifying information
    /// associated with this aggregate, such as the ID. Note that a Root also
    /// implements [`Deref`] for accessing aggregate fields.
    pub fn into_inner(this: Self) -> A {
        this.aggregate
    }
}

impl<A: Aggregate> Root<A> {
    /// Create a new Root with no existing event history.
    ///
    /// The aggregate instance will be constructed using its [`Default`].
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            last_sequence: Sequence::new(),

            aggregate: A::default(),
        }
    }

    /// Apply an Envelope to the aggregate Root.
    ///
    /// This is similar to [`Aggregate::apply`], however it also uses the
    /// envelope to validate the event's aggregate ID and update the sequence
    /// number. With this extra information, calls may fail if given an event
    /// from a different stream, or an event that has already been applied to
    /// this aggregate instance.
    pub fn try_apply<E>(this: Self, envelope: E) -> error::Result<Self>
    where
        E: Envelope,
        A::Event: DeserializeVersion,
    {
        // If the name or ID of the envelope does not match this root, the
        // event likely came from an incorrect stream.
        if envelope.name() != A::Event::name() || envelope.id() != this.id {
            return Err(Error::Invalid);
        }

        let next_sequence = envelope.sequence();
        // Once an event has been applied, earlier events are no longer valid.
        if this.last_sequence >= next_sequence {
            Err(Error::Invalid)
        } else {
            let event = envelope.deserialize::<A::Event>()?;

            Ok(Self {
                id: this.id,
                last_sequence: next_sequence,

                aggregate: this.aggregate.apply(&event),
            })
        }
    }
}

impl<A> Deref for Root<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.aggregate
    }
}
