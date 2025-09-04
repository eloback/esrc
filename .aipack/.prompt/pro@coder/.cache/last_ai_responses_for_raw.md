====
>   Info: Duration: 5m 39s 487ms | Prompt Tokens: 24,296 | Completion Tokens: 16,820 (reasoning: 7,168) 
>  Model: gpt-5 (gpt-5-2025-08-07) | Adapter: OpenAI
>  Files: Context Files: 40 | Working Files: 0 | Knowledge Files: 0

````rs
// file: src/version.rs
#[cfg(feature = "derive")]
#[doc(inline)]
pub use esrc_derive::{DeserializeVersion, SerializeVersion};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};

/// Extend the serde Deserialize trait with extra versioning info.
pub trait DeserializeVersion: for<'de> Deserialize<'de> {
    /// Deserialize the implementing type, using the specified version.
    ///
    /// In the context of this library, a version is implicitly stored with all
    /// serialized types (that use [`SerializeVersion`]). When deserialized,
    /// this version can be used to upcast events, etc.
    fn deserialize_version<'de, D>(deserializer: D, version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

/// Extend the serde Serialize trait with extra versioning info.
pub trait SerializeVersion: Serialize {
    /// Specify a version to be encoded alongside the serialized type.
    ///
    /// Where this version info is stored depends on the event store backend.
    fn version() -> usize;
}

````


````rs
// file: src/envelope.rs
use std::time::SystemTime;

#[cfg(feature = "derive")]
#[doc(inline)]
pub use esrc_derive::TryFromEnvelope;
use uuid::Uuid;

use crate::error;
use crate::event::{Event, Sequence};
use crate::version::DeserializeVersion;

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
    fn deserialize<E>(&self) -> error::Result<E>
    where
        E: DeserializeVersion + Event;
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
/// use serde::{Deserialize, Serialize};
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
/// impl TryFromEnvelope for BazEvents {
///     fn try_from_envelope(envelope: &impl Envelope) -> Result<Self, Error> {
///         if envelope.name() == FooEvent::name() {
///             Ok(BazEvents::Foo(envelope.deserialize()?))
///         } else {
///             Ok(BazEvents::Bar(envelope.deserialize()?))
///         }
///     }
/// }
/// ```
pub trait TryFromEnvelope: Sized {
    /// Attempt to deserialize the content of an Envelope and convert it.
    fn try_from_envelope(envelope: &impl Envelope) -> error::Result<Self>;
}

impl<E> TryFromEnvelope for E
where
    E: DeserializeVersion + Event,
{
    fn try_from_envelope(envelope: &impl Envelope) -> error::Result<Self> {
        envelope.deserialize()
    }
}

````


````rs
// file: src/aggregate.rs
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
        if envelope.name() != A::Event::name() || envelope.id() != this.id {
            return Err(Error::Invalid);
        }

        let next_sequence = envelope.sequence();
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

````


````rs
// file: src/event/replay.rs
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
        P: for<'de> Project<'de>;

    /// Project a subset of events.
    ///
    /// Like [`ReplayExt::rebuild`], this replays any matching events in
    /// relative order, but only with events after the given sequence number.
    async fn rebuild_after<P>(&self, projector: P, first_sequence: Sequence) -> error::Result<()>
    where
        P: for<'de> Project<'de>;
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
        P: for<'de> Project<'de>,
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
        P: for<'de> Project<'de>,
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

````


````rs
// file: src/event/subscribe.rs
use std::pin::pin;

use futures::{Stream, StreamExt};
use tracing::instrument;

use super::future::IntoSendFuture;
use super::EventGroup;
use crate::envelope;
use crate::error::{self, Error};
use crate::project::{Context, Project};

/// Subscribe to new events published to a set of event streams.
#[trait_variant::make(Send)]
pub trait Subscribe {
    /// The envelope type used by the implementing event store.
    type Envelope: envelope::Envelope;

    /// Subscribe to events published across the event streams.
    ///
    /// This includes all events identified by the EventGroup type paramter, and
    /// creates a Stream to consume these events in relative order. This method
    /// will only produce events that are published after its invocation; the
    /// Stream is infinite and will wait for new events.
    async fn subscribe<G: EventGroup>(
        &self,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send>;
}

/// Extensions for projecting newly published events.
#[trait_variant::make(Send)]
pub trait SubscribeExt: Subscribe {
    /// Subscribe to evens and project them onto the given Project type.
    ///
    /// Events published to any stream identified by the EventGroup type
    /// parameter will be included. Any errors raised by the projection will
    /// cause this method to stop processing future events.
    async fn observe<P>(&self, projector: P) -> error::Result<()>
    where
        P: for<'de> Project<'de>;
}

impl<T> SubscribeExt for T
where
    T: Subscribe + Sync,
    T::Envelope: Sync,
{
    #[instrument(skip_all, level = "debug")]
    async fn observe<P>(&self, mut projector: P) -> error::Result<()>
    where
        P: for<'de> Project<'de>,
    {
        let mut stream = pin!(self.subscribe::<P::EventGroup>().await?);
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

````


````rs
// file: src/project.rs
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
pub trait Project<'de>: Send + Clone {
    /// The event(s) that can be processed by this object.
    type EventGroup: event::EventGroup + TryFromEnvelope;
    /// The type to return as an `Err` when the projection fails.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Apply a received event, triggering implementation specific behavior.
    ///
    /// Returning an error from this method should stop further messages from
    /// being processed in the associated event store.
    async fn project<E>(
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

````


````rs
// file: src/nats/envelope.rs
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_nats::jetstream;
use serde_json::Deserializer;
use tracing::instrument;
use uuid::Uuid;

use super::header::{self, VERSION_KEY};
use super::subject::NatsSubject;
use crate::envelope::Envelope;
use crate::error::{self, Error};
use crate::event::{Event, Sequence};
use crate::version::DeserializeVersion;

/// Hold information needed to parse event types from a NATS Jetstream message.
///
/// Fields derived from the subject name and various NATS headers will be parsed
/// upon creation and stored alongside the original message.
///
/// When the envelope is dropped the message is automatically acked, and any error
/// is ignored, so the user may want to hold a reference of the message if they need
/// to manually ack or nack it.
pub struct NatsEnvelope {
    id: Uuid,
    sequence: u64,

    timestamp: i64,

    name: String,
    version: usize,
    message: jetstream::Message,
}

impl NatsEnvelope {
    /// Attempt to convert a NATS jetstream message into an Envelope instance.
    ///
    /// This requires the NATS message to:
    /// * Be published onto a subject matching the format
    ///   `<expected_prefix>.<name>.<uuid>`, where the name is a deserializable
    ///   Event's name, and the UUID is an aggregate ID.
    /// * Have an `Esrc-Version` header, which is used as the Event's version.
    #[instrument(skip_all, level = "trace")]
    pub fn try_from_message(
        expected_prefix: &str,
        message: jetstream::Message,
    ) -> error::Result<Self> {
        let NatsSubject::Aggregate(name, id) =
            NatsSubject::try_from_str(expected_prefix, message.subject.as_str())?
        else {
            return Err(Error::Invalid);
        };

        let version = header::get(&message, VERSION_KEY)
            .ok_or(Error::Invalid)?
            .parse::<usize>()
            .map_err(|e| Error::Format(e.into()))?;
        let (sequence, timestamp) = {
            let info = message.info().map_err(Error::Internal)?;
            (info.stream_sequence, info.published.unix_timestamp())
        };

        Ok(Self {
            id,
            sequence,

            timestamp,

            name: name.into_owned(),
            version,
            message,
        })
    }
}

// ack the message on envelope drop automatically
impl Drop for NatsEnvelope {
    fn drop(&mut self) {
        let message = self.message.clone();
        tokio::spawn(async move {
            let _ = message.ack().await;
        });
    }
}

impl Envelope for NatsEnvelope {
    fn id(&self) -> Uuid {
        self.id
    }

    fn sequence(&self) -> Sequence {
        self.sequence.into()
    }

    fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.timestamp as u64)
    }

    fn name(&self) -> &str {
        &self.name
    }

    #[instrument(skip_all, level = "trace")]
    fn deserialize<E>(&self) -> error::Result<E>
    where
        E: DeserializeVersion + Event,
    {
        if self.name != E::name() {
            return Err(Error::Invalid);
        }

        let mut deserializer = Deserializer::from_slice(&self.message.payload);
        E::deserialize_version(&mut deserializer, self.version).map_err(|e| Error::Format(e.into()))
    }
}

````


````rs
// file: src/kurrent/envelope.rs
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kurrentdb::ResolvedEvent;
use serde_json::Deserializer;
use tracing::instrument;
use uuid::Uuid;

use crate::envelope::Envelope;
use crate::error::{self, Error};
use crate::event::{Event, Sequence};
use crate::kurrent::header::VERSION_KEY;
use crate::kurrent::subject::KurrentSubject;
use crate::version::DeserializeVersion;

/// Hold information needed to parse event types from a kurrentdb event.
///
/// Fields derived from the subject name and various other headers will be parsed
/// upon creation and stored alongside the original event.
pub struct KurrentEnvelope {
    id: Uuid,
    sequence: u64,

    timestamp: i64,

    name: String,
    version: usize,
    resolved_event: ResolvedEvent,
}

impl KurrentEnvelope {
    /// Attempt to convert a KurrentDB event into an Envelope instance.
    ///
    /// This requires the event to:
    /// * Have an `Esrc-Version` header, which is used as the Event's version.
    #[instrument(skip_all, level = "trace")]
    pub fn try_from_message(message: ResolvedEvent) -> error::Result<Self> {
        let event = message.get_original_event();
        let KurrentSubject::Aggregate(name, id) = KurrentSubject::try_from_str(event.stream_id())?
        else {
            return Err(Error::Invalid);
        };
        let metadata: std::collections::HashMap<String, String> =
            serde_json::from_slice(&event.custom_metadata).map_err(|e| Error::Format(e.into()))?;
        let version = metadata
            .get(VERSION_KEY)
            .ok_or(Error::Invalid)?
            .parse::<usize>()
            .map_err(|e| Error::Format(e.into()))?;

        let sequence = event.position.commit;
        let timestamp = event.created.timestamp();

        Ok(Self {
            id,
            sequence,

            timestamp,

            name: name.into_owned(),
            version,
            resolved_event: message,
        })
    }
}

impl Envelope for KurrentEnvelope {
    fn id(&self) -> Uuid {
        self.id
    }

    fn sequence(&self) -> Sequence {
        self.sequence.into()
    }

    fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.timestamp as u64)
    }

    fn name(&self) -> &str {
        &self.name
    }

    #[instrument(skip_all, level = "trace")]
    fn deserialize<E>(&self) -> error::Result<E>
    where
        E: DeserializeVersion + Event,
    {
        if self.name != E::name() {
            return Err(Error::Invalid);
        }

        let mut deserializer =
            Deserializer::from_slice(&self.resolved_event.get_original_event().data);
        E::deserialize_version(&mut deserializer, self.version).map_err(|e| Error::Format(e.into()))
    }
}

````


````rs
// file: derive/src/envelope.rs
use darling::FromVariant;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{DeriveInput, Error, Fields, Ident, Variant};

use crate::util::variant;

#[derive(FromVariant)]
#[darling(attributes(esrc))]
struct TryFromEnvelopeArgs {
    #[darling(default)]
    pub ignore: bool,
}

pub fn derive_try_from_envelope(input: DeriveInput) -> Result<TokenStream, Error> {
    let variants = variant::try_collect(&input, |arg: &TryFromEnvelopeArgs| !arg.ignore)?;

    let name = input.ident;
    let envelope = Ident::new("envelope", Span::call_site());

    let variant_branches = variants
        .into_iter()
        .map(|v| get_deserialize_branch(&name, &envelope, v))
        .collect::<Result<Vec<_>, Error>>()?;

    let (impl_generics, ty_generics, clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::esrc::envelope::TryFromEnvelope for #name #ty_generics #clause {
            fn try_from_envelope(
                #envelope: &impl ::esrc::envelope::Envelope
            ) -> ::esrc::error::Result<Self> {
                if false {
                    unreachable!()
                }
                #(#variant_branches)*
                else {
                    Err(::esrc::error::Error::Invalid)
                }
            }
        }
    })
}

fn get_deserialize_branch(
    name: &Ident,
    envelope: &Ident,
    variant: Variant,
) -> Result<TokenStream, Error> {
    let variant_name = variant.ident.clone();
    if variant.fields.len() != 1 {
        return Err(Error::new(
            variant.fields.span(),
            "TryFromEnvelope variant should have one field",
        ));
    }

    let field_ty = variant.fields.iter().next().unwrap().ty.clone();
    let field_ctor = match variant.fields {
        Fields::Named(fields) => {
            let field_name = fields.named.into_iter().next().unwrap().ident;

            quote! {
                #name::#variant_name {
                    #field_name: #envelope.deserialize()?
                }
            }
        },
        Fields::Unnamed(_) => quote! {
            #name::#variant_name(#envelope.deserialize()?)
        },
        _ => return Err(Error::new(variant.span(), "bad enum field")),
    };

    Ok(quote! {
        else if #envelope.name() == <#field_ty as ::esrc::event::Event>::name() {
            Ok(#field_ctor)
        }
    })
}

````


````rs
// file: derive/src/version.rs
use darling::{FromDeriveInput, FromMeta};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Error, Ident, Lifetime};

#[derive(Default, FromMeta)]
pub struct SerdeMeta {
    pub version: Option<usize>,
    pub previous_version: Option<String>,
}

pub fn derive_deserialize_version(input: DeriveInput) -> Result<TokenStream, Error> {
    let args = super::EsrcAttributes::from_derive_input(&input).map_err(Error::from)?;
    let name = input.ident;

    let version = Ident::new("version", Span::call_site());
    let deserializer = Ident::new("deserializer", Span::call_site());

    let previous = args
        .serde
        .previous_version
        .map(|p| get_from_previous(p, &version, &deserializer))
        .unwrap_or(quote! {
            Err(<D::Error as ::serde::de::Error>::custom("unknown version"))
        });

    // Collect lifetime bounds so that the method-level deserializer lifetime
    // outlives all lifetimes declared on the type (supports borrowed fields).
    let method_lt = Lifetime::new("'__esrc_de", Span::mixed_site());
    let lifetime_bounds = input
        .generics
        .lifetimes()
        .map(|lt| {
            let sub = &lt.lifetime;
            quote! { #method_lt: #sub }
        })
        .collect::<Vec<_>>();

    let (impl_generics, ty_generics, clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::esrc::version::DeserializeVersion for #name #ty_generics #clause {
            fn deserialize_version<#method_lt, D>(#deserializer: D, #version: usize) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<#method_lt>,
                #(#lifetime_bounds,)*
            {
                if #version == <Self as ::esrc::version::SerializeVersion>::version() {
                    <Self as ::serde::Deserialize>::deserialize(#deserializer)
                } else {
                    #previous
                }
            }
        }
    })
}

pub fn derive_serialize_version(input: DeriveInput) -> Result<TokenStream, Error> {
    let args = super::EsrcAttributes::from_derive_input(&input).map_err(Error::from)?;
    let version = args.serde.version.unwrap_or(1);

    let name = input.ident;
    let (impl_generics, ty_generics, clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::esrc::version::SerializeVersion for #name #ty_generics #clause {
            fn version() -> usize {
                #version
            }
        }
    })
}

fn get_from_previous(previous_name: String, version: &Ident, deserializer: &Ident) -> TokenStream {
    let previous = Ident::new(previous_name.as_str(), Span::call_site());

    quote! {
        <#previous as ::esrc::version::DeserializeVersion>::deserialize_version(
            #deserializer,
            #version,
        )
        .map(Into::into)
    }
}

````


````rs
// file: derive/tests/fixtures/event.rs
use esrc::event::Event;
use esrc::version::DeserializeVersion;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize, PartialEq)]
pub struct FooEvent;

#[derive(Debug, Deserialize, PartialEq)]
pub struct BarEvent;

#[derive(Debug, Deserialize, PartialEq)]
pub struct LifetimeEvent<'a> {
    pub local_name: &'a str,
}

impl Event for FooEvent {
    fn name() -> &'static str {
        "Foo"
    }
}

impl DeserializeVersion for FooEvent {
    fn deserialize_version<'de, D>(_deserializer: D, _version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(FooEvent {})
    }
}

impl Event for BarEvent {
    fn name() -> &'static str {
        "Bar"
    }
}

impl DeserializeVersion for BarEvent {
    fn deserialize_version<'de, D>(_deserializer: D, _version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(BarEvent {})
    }
}

impl<'a> Event for LifetimeEvent<'a> {
    fn name() -> &'static str {
        "Lifetime"
    }
}

impl<'a> DeserializeVersion for LifetimeEvent<'a> {
    fn deserialize_version<'de, D>(_deserializer: D, _version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
        'de: 'a,
    {
        Ok(LifetimeEvent {
            local_name: "LocalLifetime",
        })
    }
}

````


````rs
// file: derive/tests/fixtures/envelope.rs
use std::time::SystemTime;

use esrc::envelope::Envelope;
use esrc::error;
use esrc::event::Sequence;
use esrc::version::DeserializeVersion;
use uuid::Uuid;

use super::version::unit_deserializer;

pub struct EmptyEnvelope {
    name: &'static str,
}

#[allow(dead_code)]
impl EmptyEnvelope {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl Envelope for EmptyEnvelope {
    fn id(&self) -> Uuid {
        Uuid::nil()
    }

    fn sequence(&self) -> Sequence {
        Sequence::new()
    }

    fn timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    fn name(&self) -> &str {
        self.name
    }

    fn deserialize<E>(&self) -> error::Result<E>
    where
        E: DeserializeVersion,
    {
        E::deserialize_version(unit_deserializer(), 1).map_err(|_| error::Error::Invalid)
    }
}

````


````rs
// file: derive/tests/test_envelope.rs
use esrc::envelope::TryFromEnvelope;
use esrc::event::Event;
use esrc::version::DeserializeVersion;
use esrc_derive::TryFromEnvelope;

mod fixtures;

use fixtures::envelope::EmptyEnvelope;
use fixtures::event::{BarEvent, FooEvent, LifetimeEvent};

#[test]
#[allow(unused)]
fn try_from_envelope() {
    #[derive(Debug, PartialEq, TryFromEnvelope)]
    enum TestGroup {
        Foo { foo: FooEvent },
        Bar(BarEvent),
    }

    let envelope = EmptyEnvelope::new(FooEvent::name());

    let expected = TestGroup::Foo { foo: FooEvent };
    let actual = TestGroup::try_from_envelope(&envelope).unwrap();

    assert_eq!(expected, actual);
}

#[test]
#[allow(unused)]
fn try_from_envelope_ignore() {
    #[derive(Debug, PartialEq, TryFromEnvelope)]
    enum TestGroup {
        Foo { foo: FooEvent },
        Bar(BarEvent),
    }

    let envelope = EmptyEnvelope::new(BarEvent::name());

    let expected = TestGroup::Bar(BarEvent);
    let actual = TestGroup::try_from_envelope(&envelope).unwrap();

    assert_eq!(expected, actual);
}

#[test]
#[allow(unused)]
fn try_from_envelope_lifetime() {
    #[derive(Debug, PartialEq, TryFromEnvelope)]
    enum TestGroup<'de, T>
    where
        T: Event + DeserializeVersion,
    {
        Other(T),
        Lifetime(LifetimeEvent<'de>),
    }

    let envelope = EmptyEnvelope::new(LifetimeEvent::name());

    let expected = TestGroup::<FooEvent>::Lifetime(LifetimeEvent {
        local_name: "LocalLifetime",
    });
    let actual = TestGroup::<FooEvent>::try_from_envelope(&envelope).unwrap();

    assert_eq!(expected, actual);
}

````


````rs
// file: derive/tests/test_version.rs
use std::marker::PhantomData;

use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc_derive::{DeserializeVersion, SerializeVersion};
use serde::{Deserialize, Serialize};

mod fixtures;
use fixtures::version::unit_deserializer;

#[test]
#[allow(unused)]
fn deserialize_version() {
    #[derive(Debug, DeserializeVersion, PartialEq, Serialize)]
    struct TestEvent;

    impl<'de> Deserialize<'de> for TestEvent {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(TestEvent)
        }
    }

    impl SerializeVersion for TestEvent {
        fn version() -> usize {
            1
        }
    }

    let expected = TestEvent;
    let actual = TestEvent::deserialize_version(unit_deserializer(), 1);
    let invalid = TestEvent::deserialize_version(unit_deserializer(), 2);

    assert_eq!(expected, actual.unwrap());
    assert!(invalid.is_err());
}

#[test]
#[allow(unused)]
fn deserialize_version_lifetime() {
    #[derive(Debug, DeserializeVersion, PartialEq, Serialize)]
    struct TestEvent<'a>(PhantomData<&'a ()>);

    impl<'a, 'de: 'a> Deserialize<'de> for TestEvent<'a> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(TestEvent(PhantomData::default()))
        }
    }

    impl<'a> SerializeVersion for TestEvent<'a> {
        fn version() -> usize {
            2
        }
    }

    let expected = TestEvent(PhantomData::default());
    let actual = TestEvent::deserialize_version(unit_deserializer(), 2);

    assert_eq!(expected, actual.unwrap());
}

#[test]
#[allow(unused)]
fn deserialize_version_previous() {
    #[derive(Debug, DeserializeVersion, PartialEq, Serialize)]
    struct TestEvent1<'a>(usize, PhantomData<&'a ()>);

    #[derive(Debug, DeserializeVersion, PartialEq, Serialize)]
    #[esrc(serde(previous_version = "TestEvent1"))]
    struct TestEvent2<'a>(usize, PhantomData<&'a ()>);

    impl<'a, 'de: 'a> Deserialize<'de> for TestEvent1<'a> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(TestEvent1(11, PhantomData::default()))
        }
    }

    impl<'a, 'de: 'a> Deserialize<'de> for TestEvent2<'a> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(TestEvent2(22, PhantomData::default()))
        }
    }

    impl<'a> SerializeVersion for TestEvent1<'a> {
        fn version() -> usize {
            1
        }
    }

    impl<'a> SerializeVersion for TestEvent2<'a> {
        fn version() -> usize {
            2
        }
    }

    impl<'a> From<TestEvent1<'a>> for TestEvent2<'a> {
        fn from(value: TestEvent1) -> Self {
            TestEvent2(44, PhantomData::default())
        }
    }

    let expected = TestEvent2(44, PhantomData::default());
    let actual = TestEvent2::deserialize_version(unit_deserializer(), 1);

    assert_eq!(expected, actual.unwrap());
}

#[test]
#[allow(unused)]
fn serialize_version() {
    #[derive(Serialize, SerializeVersion)]
    #[esrc(serde(version = 2))]
    struct TestEvent;

    assert_eq!(2, TestEvent::version());
}

#[test]
#[allow(unused)]
fn serialize_version_default() {
    #[derive(Serialize, SerializeVersion)]
    struct TestEvent;

    assert_eq!(1, TestEvent::version());
}

````

