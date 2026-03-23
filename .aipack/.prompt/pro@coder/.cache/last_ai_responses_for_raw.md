====
>   Info: Duration: 1m 58s | ~$0.1898 | Prompt Tokens: 18,998 (cached: 0, cache_creation: 0) | Completion Tokens: 8,856 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 12 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Here is the plan:

1. Create sub-modules inside `crates/esrc-cqrs/src/nats/`:
   - `command/` - holds `aggregate_command_handler.rs`
   - `query/` - holds `live_view_query.rs`, `memory_view_query.rs`
   - `client/` - holds `client.rs`

2. Keep `command_dispatcher`, `query_dispatcher`, `projector_runner`, `durable_projector_handler` at the root of `nats/` since they are the core dispatch infrastructure.

3. Update `mod.rs` to reflect the new sub-module structure and only re-export the dispatcher/runner types at the root level.

4. Update the cafe example imports.

<FILE_CHANGES>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/command/mod.rs">
/// Aggregate command handler wiring: maps a typed command to an aggregate and writes events.
pub mod aggregate_command_handler;

pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/command/aggregate_command_handler.rs">
use std::marker::PhantomData;

use esrc::aggregate::{Aggregate, Root};
use esrc::error;
use esrc::event::publish::PublishExt;
use esrc::event::replay::ReplayOneExt;
use esrc::nats::NatsStore;
use esrc::version::{DeserializeVersion, SerializeVersion};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::command::CommandHandler;

/// A standard command envelope sent over NATS.
///
/// The command payload wraps the aggregate ID and the serialized command body.
/// Both the ID and the command are encoded as JSON.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandEnvelope<C> {
    /// The ID of the aggregate instance this command targets.
    pub id: Uuid,
    /// The actual command to process.
    pub command: C,
}

/// A standard reply envelope returned after processing a command.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandReply {
    /// The aggregate ID that was modified.
    pub id: Uuid,
    /// Whether the command succeeded.
    pub success: bool,
    /// The structured CQRS error, present only when `success` is false.
    pub error: Option<crate::Error>,
}

/// A generic [`CommandHandler`] implementation for NATS-backed aggregates.
///
/// This handler:
/// 1. Deserializes the incoming payload as a [`CommandEnvelope<A::Command>`].
/// 2. Loads the aggregate using [`ReplayOneExt::read`].
/// 3. Processes and writes the command using [`PublishExt::try_write`].
/// 4. Returns a serialized [`CommandReply`].
///
/// `A` is the aggregate type. `A::Command` must implement `Deserialize` and
/// `A::Event` must implement both `SerializeVersion` and `DeserializeVersion`.
pub struct AggregateCommandHandler<A>
where
    A: Aggregate,
{
    /// The name used to route commands to this handler.
    ///
    /// Convention: `<AggregateName>.<CommandName>` or just `<AggregateName>`.
    handler_name: &'static str,
    _phantom: PhantomData<A>,
}

impl<A> AggregateCommandHandler<A>
where
    A: Aggregate,
{
    /// Create a new handler with the given routing name.
    pub fn new(handler_name: &'static str) -> Self {
        Self {
            handler_name,
            _phantom: PhantomData,
        }
    }
}

impl<A> CommandHandler<NatsStore> for AggregateCommandHandler<A>
where
    A: Aggregate + Send + Sync + 'static,
    A::Command: for<'de> Deserialize<'de> + Send,
    A::Event: SerializeVersion + DeserializeVersion + Send,
    A::Error: Serialize,
{
    fn name(&self) -> &'static str {
        self.handler_name
    }

    async fn handle<'a>(
        &'a self,
        store: &'a mut NatsStore,
        payload: &'a [u8],
    ) -> error::Result<Vec<u8>> {
        let envelope: CommandEnvelope<A::Command> =
            serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;

        let root: Root<A> = store.read(envelope.id).await?;
        let agg_id = envelope.id;
        let root = store.try_write(root, envelope.command, None).await;

        let reply = match root {
            Ok(written) => CommandReply {
                id: Root::id(&written),
                success: true,
                error: None,
            },
            Err(e) => {
                // Convert the esrc error into a serializable cqrs_error::Error.
                // For the External variant the aggregate's Error must implement
                // Serialize (enforced by the trait bound above). We serialize it
                // into a serde_json::Value before boxing so that from_esrc_error
                // can recover the structured value on the other side.
                let cqrs_err = convert_esrc_error::<A>(e);
                CommandReply {
                    id: agg_id,
                    success: false,
                    error: Some(cqrs_err),
                }
            },
        };
        serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
    }
}

/// Convert an [`esrc::error::Error`] into a [`cqrs_error::Error`], serializing
/// the aggregate's domain error for the `External` variant.
///
/// The `External` source produced by `try_write` is the aggregate's own `Error`
/// type boxed as `Box<dyn std::error::Error + Send + Sync>`. We downcast it
/// back to `A::Error` and serialize it. If the downcast fails (which would be a
/// framework bug), we fall back to the Display representation. If serialization
/// fails, we panic because a non-serializable aggregate error is a programming
/// error when using the CQRS framework.
fn convert_esrc_error<A>(err: esrc::error::Error) -> crate::Error
where
    A: Aggregate,
    A::Error: Serialize,
{
    match err {
        esrc::error::Error::Internal(e) => crate::Error::Internal(e.to_string()),
        esrc::error::Error::External(e) => {
            let value = match e.downcast::<A::Error>() {
                Ok(agg_err) => serde_json::to_value(&*agg_err)
                    .expect("aggregate Error must be serializable when used with esrc-cqrs"),
                Err(e) => serde_json::Value::String(e.to_string()),
            };
            crate::Error::External(value)
        },
        esrc::error::Error::Format(e) => crate::Error::Format(e.to_string()),
        esrc::error::Error::Invalid => crate::Error::Invalid,
        esrc::error::Error::Conflict => crate::Error::Conflict,
    }
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/query/mod.rs">
/// Live view query handler: replays events on each request to build a View.
pub mod live_view_query;
/// Memory view projector and query handler: keeps a View per aggregate ID in memory.
pub mod memory_view_query;

pub use live_view_query::LiveViewQuery;
pub use memory_view_query::{MemoryView, MemoryViewQuery};
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/query/live_view_query.rs">
use std::marker::PhantomData;

use esrc::error::{self, Error};
use esrc::event::replay::ReplayOne;
use esrc::nats::NatsStore;
use esrc::view::View;
use esrc::Envelope;
use futures::StreamExt;
use serde::Serialize;

use crate::nats::query_dispatcher::{QueryEnvelope, QueryReply};
use crate::query::QueryHandler;

/// A [`QueryHandler`] that replays an event stream on each request to build a
/// [`View`] and return a projected read model as the query response.
///
/// On every incoming query, `LiveViewQuery` replays the full event history for
/// the requested aggregate ID, folds all events into a fresh `V` instance
/// starting from `V::default()`, applies the projection function, and returns
/// the serialized result inside a [`QueryReply`].
///
/// This is suitable for views where replaying on demand is acceptable (e.g.,
/// small streams or low-throughput queries). For higher-throughput scenarios,
/// prefer [`MemoryViewQuery`] which keeps an in-memory projection updated by a
/// running projector.
///
/// `V` is the [`View`] type to build from replayed events.
/// `R` is the read-model type returned to the caller; it must implement
/// [`serde::Serialize`].
pub struct LiveViewQuery<V, R> {
    /// The unique handler name used to route queries to this handler.
    handler_name: &'static str,
    /// Projects a built view into the serializable response type.
    projection: fn(&V) -> R,
    _phantom: PhantomData<(V, R)>,
}

impl<V, R> LiveViewQuery<V, R>
where
    V: View,
    R: Serialize,
{
    /// Create a new handler with the given routing name and projection function.
    ///
    /// `handler_name` is used to route incoming query messages to this handler.
    /// `projection` converts the built `V` into the serializable response `R`.
    pub fn new(handler_name: &'static str, projection: fn(&V) -> R) -> Self {
        Self {
            handler_name,
            projection,
            _phantom: PhantomData,
        }
    }
}

impl<V> LiveViewQuery<V, V>
where
    V: View + Clone + Serialize,
{
    /// Create a new handler with the given routing name
    /// that returns the entire view as the response, without a separate projection step.
    pub fn new_for_serializable_view(handler_name: &'static str) -> Self {
        Self {
            handler_name,
            projection: |view| view.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<V, R> QueryHandler<NatsStore> for LiveViewQuery<V, R>
where
    V: View + Send + Sync + 'static,
    V::Event: esrc::version::DeserializeVersion + Send,
    R: Serialize + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        self.handler_name
    }

    async fn handle<'a>(
        &'a self,
        store: &'a NatsStore,
        payload: &'a [u8],
    ) -> error::Result<Vec<u8>> {
        let envelope: QueryEnvelope =
            serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;

        // Replay the full event history for the requested aggregate ID, starting from sequence 0.
        let mut stream = store
            .replay_one::<V::Event>(envelope.id, esrc::event::Sequence::new())
            .await?;

        let mut view = V::default();
        while let Some(result) = stream.next().await {
            let nats_envelope = result?;
            let event = nats_envelope
                .deserialize::<V::Event>()
                .map_err(|e| Error::Format(format!("{e}").into()))?;
            view = view.apply(&event);
        }

        let data = serde_json::to_value((self.projection)(&view))
            .map_err(|e| esrc::error::Error::Format(e.into()))?;

        let reply = QueryReply {
            success: true,
            data: Some(data),
            error: None,
        };
        serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
    }
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/query/memory_view_query.rs">
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use esrc::envelope::Envelope;
use esrc::error;
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use esrc::view::View;
use serde::Serialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::nats::query_dispatcher::{QueryEnvelope, QueryReply};
use crate::query::QueryHandler;

/// An in-memory projection that keeps a [`View`] per aggregate ID.
///
/// `MemoryView<V>` implements [`Project`] so it can be registered as a
/// projector. It is also the shared backing store for [`MemoryViewQuery`].
///
/// Multiple `MemoryViewQuery` instances can share the same `MemoryView` handle
/// because the internal map is wrapped in an `Arc<RwLock<...>>`.
///
/// `V` must implement [`View`] and [`Clone`] so that a snapshot can be taken
/// for the projection function without holding the write lock.
#[derive(Clone)]
pub struct MemoryView<V> {
    views: Arc<RwLock<HashMap<Uuid, V>>>,
}

impl<V> Default for MemoryView<V> {
    fn default() -> Self {
        Self {
            views: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<V> MemoryView<V> {
    /// Create a new, empty `MemoryView`.
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("memory view projection error")]
struct MemoryViewError;

impl<V> Project for MemoryView<V>
where
    V: View + Clone + Send + Sync + 'static,
    V::Event: esrc::version::DeserializeVersion + Send,
{
    type EventGroup = V::Event;
    type Error = std::convert::Infallible;

    async fn project<'de, E>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: Envelope + Sync,
    {
        let id = Context::id(&context);
        let event = Context::into_inner(context);

        let mut map = self.views.write().await;
        let view = map.entry(id).or_insert_with(V::default);
        // Apply the event in-place by temporarily swapping the value out.
        let current = std::mem::replace(view, V::default());
        *view = current.apply(&event);

        Ok(())
    }
}

/// A [`QueryHandler`] that reads from a [`MemoryView`] to answer queries.
///
/// On every incoming query, `MemoryViewQuery` looks up the current `V` for the
/// requested aggregate ID in the shared in-memory map, applies the projection
/// function, and returns the serialized result inside a [`QueryReply`].
///
/// If the aggregate ID has never been seen by the projector, `V::default()` is
/// used, which matches the semantics of an aggregate with no events applied.
///
/// `V` is the [`View`] type held in memory.
/// `R` is the read-model type returned to the caller; it must implement
/// [`serde::Serialize`].
pub struct MemoryViewQuery<V, R> {
    /// The unique handler name used to route queries to this handler.
    handler_name: &'static str,
    /// The shared in-memory view store.
    memory_view: MemoryView<V>,
    /// Projects a snapshot of the view into the serializable response type.
    projection: fn(&V) -> R,
    _phantom: PhantomData<R>,
}

impl<V, R> MemoryViewQuery<V, R>
where
    V: View + Clone,
    R: Serialize,
{
    /// Create a new handler with the given routing name, shared memory view, and projection function.
    ///
    /// `handler_name` is used to route incoming query messages to this handler.
    /// `memory_view` is the shared `MemoryView` instance that must also be registered
    /// as a projector so it receives events.
    /// `projection` converts a view snapshot into the serializable response `R`.
    pub fn new(
        handler_name: &'static str,
        memory_view: MemoryView<V>,
        projection: fn(&V) -> R,
    ) -> Self {
        Self {
            handler_name,
            memory_view,
            projection,
            _phantom: PhantomData,
        }
    }
}

impl<V> MemoryViewQuery<V, V>
where
    V: View + Clone + Serialize,
{
    /// Create a new handler with the given routing name
    /// that returns the entire view as the response, without a separate projection step.
    pub fn new_for_serializable_view(
        handler_name: &'static str,
        memory_view: MemoryView<V>,
    ) -> Self {
        Self {
            handler_name,
            memory_view,
            projection: |view| view.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<V, R> QueryHandler<NatsStore> for MemoryViewQuery<V, R>
where
    V: View + Clone + Send + Sync + 'static,
    V::Event: esrc::version::DeserializeVersion + Send,
    R: Serialize + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        self.handler_name
    }

    async fn handle<'a>(
        &'a self,
        _store: &'a NatsStore,
        payload: &'a [u8],
    ) -> error::Result<Vec<u8>> {
        let envelope: QueryEnvelope =
            serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;

        // Take a snapshot under a read lock to avoid holding the lock during serialization.
        let snapshot: V = {
            let map = self.memory_view.views.read().await;
            map.get(&envelope.id).cloned().unwrap_or_default()
        };

        let data = serde_json::to_value((self.projection)(&snapshot))
            .map_err(|e| esrc::error::Error::Format(e.into()))?;

        let reply = QueryReply {
            success: true,
            data: Some(data),
            error: None,
        };
        serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
    }
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/client/mod.rs">
/// High-level CQRS client for ergonomic command and query dispatch.
mod cqrs_client;

pub use cqrs_client::CqrsClient;
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/client/cqrs_client.rs">
use async_nats::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use esrc::error::{self, Error};

use crate::nats::command::aggregate_command_handler::{CommandEnvelope, CommandReply};
use crate::nats::command_dispatcher::command_subject;
use crate::nats::query_dispatcher::{QueryEnvelope, QueryReply, query_subject};

/// High-level CQRS client that removes boilerplate from command and query dispatch.
///
/// `CqrsClient` wraps an `async_nats::Client` and handles envelope construction,
/// serialization, subject building, and reply deserialization internally. Callers
/// only supply the service name, handler name, aggregate ID, and payload.
///
/// # Command dispatch
///
/// ```rust,ignore
/// let reply = client
///     .send_command("my-service", "MyAggregate", aggregate_id, my_command)
///     .await?;
/// ```
///
/// # Query dispatch
///
/// ```rust,ignore
/// let state: MyState = client
///     .send_query("my-service", "MyAggregate.GetState", aggregate_id)
///     .await?;
/// ```
#[derive(Clone, Debug)]
pub struct CqrsClient {
    inner: Client,
}

impl CqrsClient {
    /// Create a new `CqrsClient` wrapping the given NATS client.
    pub fn new(client: Client) -> Self {
        Self { inner: client }
    }

    /// Return a reference to the underlying `async_nats::Client`.
    pub fn inner(&self) -> &Client {
        &self.inner
    }

    /// Send a command to a handler and return the raw [`CommandReply`].
    ///
    /// The envelope is constructed and serialized internally. The subject is
    /// built from `service_name` and `handler_name` using [`command_subject`].
    ///
    /// # Errors
    ///
    /// Returns an [`esrc::error::Error::Internal`] if the NATS request fails or
    /// the reply cannot be deserialized. A successful return does not imply
    /// `reply.success == true`; the caller should inspect [`CommandReply::success`].
    pub async fn send_command<C>(
        &self,
        service_name: &str,
        handler_name: &str,
        id: Uuid,
        command: C,
    ) -> error::Result<CommandReply>
    where
        C: Serialize,
    {
        let envelope = CommandEnvelope { id, command };
        let payload =
            serde_json::to_vec(&envelope).map_err(|e| Error::Format(e.into()))?;
        let subject = command_subject(service_name, handler_name);

        let msg = self
            .inner
            .request(subject, payload.into())
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        serde_json::from_slice::<CommandReply>(&msg.payload)
            .map_err(|e| Error::Format(e.into()))
    }

    /// Send a command and return `Ok(reply.id)` on success, or convert the
    /// [`CommandReply`] error into an [`esrc::error::Error`] on failure.
    ///
    /// This is a convenience wrapper around [`send_command`] for callers that
    /// want to propagate command failures as `Result::Err` rather than
    /// inspecting the reply manually.
    ///
    /// [`send_command`]: CqrsClient::send_command
    pub async fn dispatch_command<C>(
        &self,
        service_name: &str,
        handler_name: &str,
        id: Uuid,
        command: C,
    ) -> error::Result<Uuid>
    where
        C: Serialize,
    {
        let reply = self.send_command(service_name, handler_name, id, command).await?;
        if reply.success {
            Ok(reply.id)
        } else {
            let msg = reply
                .error
                .as_ref()
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "command failed".to_string());
            Err(Error::Internal(msg.into()))
        }
    }

    /// Send a query to a handler and return the raw [`QueryReply`].
    ///
    /// The envelope is constructed and serialized internally. The subject is
    /// built from `service_name` and `handler_name` using [`query_subject`].
    ///
    /// # Errors
    ///
    /// Returns an [`esrc::error::Error::Internal`] if the NATS request fails or
    /// the reply cannot be deserialized. A successful return does not imply
    /// `reply.success == true`; the caller should inspect [`QueryReply::success`].
    pub async fn send_query(
        &self,
        service_name: &str,
        handler_name: &str,
        id: Uuid,
    ) -> error::Result<QueryReply>
    where
    {
        let envelope = QueryEnvelope { id };
        let payload =
            serde_json::to_vec(&envelope).map_err(|e| Error::Format(e.into()))?;
        let subject = query_subject(service_name, handler_name);

        let msg = self
            .inner
            .request(subject, payload.into())
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        serde_json::from_slice::<QueryReply>(&msg.payload)
            .map_err(|e| Error::Format(e.into()))
    }

    /// Send a query and deserialize the result directly into `T`.
    ///
    /// This is a convenience wrapper around [`send_query`] for callers that
    /// want a typed result rather than a raw [`QueryReply`]. Returns
    /// `Err(Error::Internal(...))` when `reply.success` is false.
    ///
    /// # Errors
    ///
    /// Returns an error if the NATS request fails, the reply cannot be
    /// deserialized, or the `data` field is absent even though `success` is true.
    ///
    /// [`send_query`]: CqrsClient::send_query
    pub async fn dispatch_query<T>(
        &self,
        service_name: &str,
        handler_name: &str,
        id: Uuid,
    ) -> error::Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let reply = self.send_query(service_name, handler_name, id).await?;
        if !reply.success {
            let msg = reply
                .error
                .as_ref()
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "query failed".to_string());
            return Err(Error::Internal(msg.into()));
        }
        let data = reply.data.ok_or_else(|| {
            Error::Internal("query succeeded but returned no data".into())
        })?;
        serde_json::from_value::<T>(data).map_err(|e| Error::Format(e.into()))
    }
}

impl From<Client> for CqrsClient {
    fn from(client: Client) -> Self {
        Self::new(client)
    }
}
</FILE_NEW>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
 //! NATS-backed CQRS dispatcher and projector runner.
 //!
 //! # Command Dispatcher
 //!
 //! Commands use NATS **core request/reply**: the dispatcher creates a service
 //! group on the JetStream context and listens on subjects of the form
 //! `<prefix>.cmd.<handler_name>`. Each incoming request is dispatched to the
 //! matching [`CommandHandler`], and the reply is sent back to the caller.
 //!
 //! This is the correct transport choice for commands because:
 //! * Commands are point-in-time requests that expect an immediate acknowledgment.
 //! * Core NATS request/reply is low-latency and naturally load-balances across
 //!   multiple service instances via queue groups.
 //! * There is no need to persist commands; only the resulting events are durable.
 //!
 //! # Projector Runner
 //!
 //! Projectors use NATS **JetStream durable pull consumers** (the same mechanism
 //! as the existing `Subscribe` / `durable_observe` in `NatsStore`). Each
 //! projector runs as an independent task and resumes from its last position
 //! across restarts using its durable consumer name.
 //!
 //! This is the correct transport choice for projectors because:
 //! * Event projections must be durable and survive process restarts.
 //! * Pull consumers allow back-pressure and fine-grained acknowledgment.
 //! * Each projector gets its own consumer position so they progress independently.
 //!
 //! # Query Dispatcher
 //!
 //! Queries use NATS **core request/reply**, the same transport as commands, but
 //! with a shared (non-exclusive) store reference because queries are read-only.
 //! The dispatcher creates a service group and listens on subjects of the form
 //! `<service_name>.<handler_name>`. Each incoming request is dispatched to the
 //! matching [`QueryHandler`], and the reply is sent back to the caller.
 //!
 //! This is the correct transport choice for queries because:
 //! * Queries are point-in-time reads that expect an immediate response.
 //! * Sharing the store across handlers avoids unnecessary cloning of connections.
 //! * Core NATS request/reply naturally load-balances across service instances.

 /// NATS command dispatcher backed by core NATS request/reply service groups.
 pub mod command_dispatcher;
 /// NATS projector runner backed by JetStream durable consumers.
 pub mod projector_runner;
 /// NATS query dispatcher backed by core NATS request/reply service groups.
 pub mod query_dispatcher;

-pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
-pub use client::CqrsClient;
-pub use command_dispatcher::NatsCommandDispatcher;
-pub use durable_projector_handler::DurableProjectorHandler;
-pub use live_view_query::LiveViewQuery;
-pub use memory_view_query::{MemoryView, MemoryViewQuery};
-pub use projector_runner::NatsProjectorRunner;
-pub use query_dispatcher::{NatsQueryDispatcher, QueryEnvelope, QueryReply};
-
-/// Aggregate command handler wiring: maps a typed command to an aggregate and writes events.
-mod aggregate_command_handler;
-/// High-level CQRS client for ergonomic command and query dispatch.
-mod client;
-/// Durable projector handler wiring: maps a projector to a durable JetStream consumer.
-mod durable_projector_handler;
-/// Live view query handler: replays events on each request to build a View.
-mod live_view_query;
-/// Memory view projector and query handler: keeps a View per aggregate ID in memory.
-mod memory_view_query;
+pub use command_dispatcher::NatsCommandDispatcher;
+pub use projector_runner::NatsProjectorRunner;
+pub use query_dispatcher::{NatsQueryDispatcher, QueryEnvelope, QueryReply};
+
+/// Aggregate command handler and envelope types.
+pub mod command;
+/// High-level CQRS client for ergonomic command and query dispatch.
+pub mod client;
+/// Query handler implementations: live-view and in-memory projections.
+pub mod query;
+
+/// Durable projector handler wiring: maps a projector to a durable JetStream consumer.
+pub mod durable_projector_handler;
+
+pub use durable_projector_handler::DurableProjectorHandler;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command_dispatcher.rs">
@@
 use super::aggregate_command_handler::CommandReply;
+use super::command::aggregate_command_handler::CommandReply;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/lib.rs">
@@
 pub use command::CommandHandler;
 pub use error::Error;
 #[cfg(feature = "nats")]
-pub use nats::CqrsClient;
+pub use nats::client::CqrsClient;
 pub use projector::ProjectorHandler;
 pub use registry::CqrsRegistry;
 pub use query::QueryHandler;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/examples/cafe/main.rs">
@@
 use esrc::nats::NatsStore;
 use esrc_cqrs::nats::{
-    AggregateCommandHandler, CommandEnvelope, CommandReply, CqrsClient, DurableProjectorHandler,
-    LiveViewQuery, MemoryView, MemoryViewQuery, NatsCommandDispatcher, NatsQueryDispatcher,
-    QueryEnvelope, QueryReply,
+    DurableProjectorHandler, NatsCommandDispatcher, NatsQueryDispatcher,
+    QueryEnvelope, QueryReply,
 };
+use esrc_cqrs::nats::client::CqrsClient;
+use esrc_cqrs::nats::command::{AggregateCommandHandler, CommandEnvelope, CommandReply};
+use esrc_cqrs::nats::query::{LiveViewQuery, MemoryView, MemoryViewQuery};
 use esrc_cqrs::CqrsRegistry;
</FILE_PATCH>

</FILE_CHANGES>

Now I need to fix the `command_dispatcher.rs` patch - it has a duplicate import line. Let me reconsider: the original file has `use super::aggregate_command_handler::CommandReply;` and I need to replace it with the new path.

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command_dispatcher.rs">
@@
-use super::aggregate_command_handler::CommandReply;
+use super::command::aggregate_command_handler::CommandReply;
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git add -A .
git commit -a -m "^ esrc-cqrs/nats - Reorganize nats module into command, query, and client sub-modules"
</suggested_git_command>

