====
>   Info: Duration: 1m 39s | ~$0.2075 | Prompt Tokens: 32,920 (cached: 0, cache_creation: 0) | Completion Tokens: 7,250 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 40 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll design and implement the `esrc-cqrs` extension crate. Let me reason through the design first.

## Design Reasoning

**Command Handler (CQRS Command Side):**
- Commands are imperative, targeted at a specific aggregate instance
- They need request/reply semantics since the caller wants confirmation
- NATS **Request/Reply** (via JetStream services or core NATS) is the natural fit
- Subject pattern: `<prefix>.cmd.<AggregateName>.<CommandName>`
- The handler loads the aggregate, processes the command, publishes the event, and replies

**Event Handler (Project side):**
- Event handlers are projectors that react to published events
- They need durable, at-least-once delivery for reliability
- NATS **JetStream durable consumers** (push or pull) are the right choice, same as the existing `Subscribe` implementation in `esrc`
- The registry manages multiple projectors attached to different event groups

**Registry Design:**
- A `CqrsRegistry` holds both command handlers and event handler projectors
- It uses the existing `esrc` traits: `Aggregate`, `Project`, `Publish`, `ReplayOne`
- Command handler: loads aggregate via `ReplayOneExt::read`, processes via `PublishExt::try_write`
- Event handler: delegates to existing `SubscribeExt::observe` or `ReplayExt::rebuild`

**Workspace structure:**
- New crate at `crates/esrc-cqrs/`
- NATS impl lives within it behind a feature flag

<FILE_CHANGES>

<FILE_NEW file_path="crates/esrc-cqrs/Cargo.toml">
[package]
name = "esrc-cqrs"
description = "CQRS command and event handler registry for the esrc crate"
readme = false
version = "0.1.0"

license.workspace = true
authors.workspace = true
repository.workspace = true

rust-version.workspace = true
edition.workspace = true

[features]
default = ["nats"]
nats = ["dep:async-nats", "dep:esrc/nats", "dep:serde_json", "dep:serde"]

[dependencies]
esrc = { path = "../..", version = "0.3.1" }

async-nats = { workspace = true, optional = true }
futures.workspace = true
serde = { workspace = true, optional = true }
serde_json = { workspace = true, optional = true }
thiserror.workspace = true
tokio.workspace = true
tracing.workspace = true
trait-variant.workspace = true
uuid.workspace = true
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/lib.rs">
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! CQRS extension for `esrc`.
//!
//! Provides a registry for command handlers and event projectors, allowing
//! structured dispatch of commands and event projections over an event store.
//!
//! # Command Handlers
//!
//! A command handler receives a typed command, loads the target aggregate,
//! processes the command, and writes the resulting event back to the store.
//! Handlers are registered by aggregate type and dispatched by subject.
//!
//! # Event Handlers (Projectors)
//!
//! Event handlers are [`esrc::project::Project`] implementors. They are
//! registered and driven by the registry, which subscribes them to the
//! relevant event streams.

/// Command handler trait and registry entry.
pub mod command;
/// Event projector handler registry entry.
pub mod projector;
/// The top-level CQRS registry that holds and drives all handlers.
pub mod registry;

/// NATS-backed implementations of the command dispatcher and projector runner.
#[cfg(feature = "nats")]
pub mod nats;

pub use command::CommandHandler;
pub use projector::ProjectorHandler;
pub use registry::CqrsRegistry;
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/command.rs">
use std::future::Future;

use esrc::error;

/// A handler for a single command type.
///
/// Implementors receive a raw byte payload (the serialized command), perform
/// all aggregate loading, command processing, and event writing, then return
/// an optional byte payload as a reply to the caller.
///
/// The generic parameter `S` is the event store type (e.g., `NatsStore`).
pub trait CommandHandler<S>: Send + Sync + 'static {
    /// The unique name for this command handler.
    ///
    /// This is used to route incoming command messages to the correct handler.
    /// The convention is `<AggregateName>.<CommandName>`.
    fn name(&self) -> &'static str;

    /// Handle a raw incoming command payload, returning a reply payload.
    ///
    /// The handler is responsible for deserializing the command, loading the
    /// aggregate, processing the command, writing the event, and serializing
    /// any reply. Returning an `Err` will cause an error reply to be sent.
    fn handle<'a>(
        &'a self,
        store: &'a mut S,
        payload: &'a [u8],
    ) -> impl Future<Output = error::Result<Vec<u8>>> + Send + 'a;
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/projector.rs">
use std::future::Future;

use esrc::error;

/// A handler that drives an event projector subscription.
///
/// Each registered projector is run as an independent task. The handler
/// encapsulates the subscription logic (durable or transient) for the
/// underlying store backend.
///
/// The generic parameter `S` is the event store type (e.g., `NatsStore`).
pub trait ProjectorHandler<S>: Send + 'static {
    /// The unique durable name for this projector.
    ///
    /// Used as the durable consumer name so that the projector resumes from
    /// its last position across restarts.
    fn name(&self) -> &'static str;

    /// Run the projector against the given store, consuming events indefinitely.
    ///
    /// This future is expected to run until an error occurs or the process
    /// shuts down. It drives the underlying [`esrc::project::Project`]
    /// implementation.
    fn run<'a>(&'a self, store: &'a S) -> impl Future<Output = error::Result<()>> + Send + 'a;
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/registry.rs">
use std::sync::Arc;

use tokio::task::JoinSet;
use tracing::instrument;

use esrc::error;

use crate::command::CommandHandler;
use crate::projector::ProjectorHandler;

/// A registry that holds command handlers and event projectors.
///
/// `S` is the event store type shared across all handlers. The store must be
/// `Clone` so that each command handler and projector can hold its own handle.
pub struct CqrsRegistry<S> {
    store: S,
    command_handlers: Vec<Arc<dyn ErasedCommandHandler<S>>>,
    projector_handlers: Vec<Arc<dyn ErasedProjectorHandler<S>>>,
}

impl<S> CqrsRegistry<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Create a new registry backed by the given store.
    pub fn new(store: S) -> Self {
        Self {
            store,
            command_handlers: Vec::new(),
            projector_handlers: Vec::new(),
        }
    }

    /// Register a command handler.
    ///
    /// The handler will be invoked when a command matching its `name()` is
    /// dispatched through the registry's command listener.
    pub fn register_command<H>(mut self, handler: H) -> Self
    where
        H: CommandHandler<S> + 'static,
    {
        self.command_handlers.push(Arc::new(handler));
        self
    }

    /// Register an event projector handler.
    ///
    /// The projector will be started as a background task when `run` is called.
    pub fn register_projector<H>(mut self, handler: H) -> Self
    where
        H: ProjectorHandler<S> + 'static,
    {
        self.projector_handlers.push(Arc::new(handler));
        self
    }

    /// Return a reference to the registered command handlers.
    pub fn command_handlers(&self) -> &[Arc<dyn ErasedCommandHandler<S>>] {
        &self.command_handlers
    }

    /// Return a reference to the registered projector handlers.
    pub fn projector_handlers(&self) -> &[Arc<dyn ErasedProjectorHandler<S>>] {
        &self.projector_handlers
    }

    /// Return a clone of the backing store.
    pub fn store(&self) -> S {
        self.store.clone()
    }

    /// Start all projectors as background tasks and return a [`JoinSet`].
    ///
    /// Each projector is spawned on the Tokio runtime. The caller can await
    /// the `JoinSet` to wait for all projectors to complete (or fail).
    #[instrument(skip_all, level = "debug")]
    pub async fn run_projectors(&self) -> error::Result<JoinSet<error::Result<()>>> {
        let mut set = JoinSet::new();

        for handler in &self.projector_handlers {
            let handler = Arc::clone(handler);
            let store = self.store.clone();
            set.spawn(async move { handler.run_erased(&store).await });
        }

        Ok(set)
    }
}

// -- Object-safe erased traits so we can store heterogeneous handlers --

/// Object-safe wrapper for [`CommandHandler`].
pub trait ErasedCommandHandler<S>: Send + Sync + 'static {
    /// The name of the command this handler processes.
    fn name(&self) -> &'static str;
    /// Handle the raw payload and return a reply.
    fn handle_erased<'a>(
        &'a self,
        store: &'a mut S,
        payload: &'a [u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<Vec<u8>>> + Send + 'a>>;
}

impl<S, H> ErasedCommandHandler<S> for H
where
    H: CommandHandler<S> + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        CommandHandler::name(self)
    }

    fn handle_erased<'a>(
        &'a self,
        store: &'a mut S,
        payload: &'a [u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<Vec<u8>>> + Send + 'a>>
    {
        Box::pin(self.handle(store, payload))
    }
}

/// Object-safe wrapper for [`ProjectorHandler`].
pub trait ErasedProjectorHandler<S>: Send + Sync + 'static {
    /// The durable name of this projector.
    fn name(&self) -> &'static str;
    /// Run the projector against the store.
    fn run_erased<'a>(
        &'a self,
        store: &'a S,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<()>> + Send + 'a>>;
}

impl<S, H> ErasedProjectorHandler<S> for H
where
    H: ProjectorHandler<S> + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        ProjectorHandler::name(self)
    }

    fn run_erased<'a>(
        &'a self,
        store: &'a S,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<()>> + Send + 'a>> {
        Box::pin(self.run(store))
    }
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/mod.rs">
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

/// NATS command dispatcher backed by core NATS request/reply service groups.
pub mod command_dispatcher;
/// NATS projector runner backed by JetStream durable consumers.
pub mod projector_runner;

pub use command_dispatcher::NatsCommandDispatcher;
pub use projector_runner::NatsProjectorRunner;
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/command_dispatcher.rs">
use std::sync::Arc;

use async_nats::service::ServiceExt;
use futures::StreamExt;
use tracing::instrument;

use esrc::error::{self, Error};
use esrc::nats::NatsStore;

use crate::registry::ErasedCommandHandler;

/// Subject prefix used for all command messages.
///
/// Full subject pattern: `<service_name>.<handler_name>`
/// where `handler_name` is the value returned by `CommandHandler::name()`.
pub const CMD_SERVICE_VERSION: &str = "0.1.0";

/// NATS command dispatcher.
///
/// Registers all command handlers as endpoints on a single NATS service,
/// using core NATS request/reply. Each handler name becomes one endpoint
/// within the service group named `<prefix>-cqrs`.
///
/// A mutable clone of the store is passed into each handler invocation so
/// that publishing events works correctly (since `Publish` requires `&mut self`).
pub struct NatsCommandDispatcher {
    /// The NATS client used to create the service.
    client: async_nats::Client,
    /// The service group name (e.g. `"myapp-cqrs"`).
    service_name: String,
}

impl NatsCommandDispatcher {
    /// Create a new dispatcher using the given NATS client and service name.
    pub fn new(client: async_nats::Client, service_name: impl Into<String>) -> Self {
        Self {
            client,
            service_name: service_name.into(),
        }
    }

    /// Start the command dispatcher and listen for incoming commands.
    ///
    /// This method creates one NATS service endpoint per registered command
    /// handler. Each endpoint is named after the handler's `name()`. The
    /// dispatcher runs until an error occurs or the NATS connection is closed.
    ///
    /// A fresh clone of `store` is passed into each request so that handlers
    /// can use `&mut store` for publishing without contention.
    #[instrument(skip_all, level = "debug")]
    pub async fn run<S>(
        &self,
        store: S,
        handlers: &[Arc<dyn ErasedCommandHandler<S>>],
    ) -> error::Result<()>
    where
        S: Clone + Send + Sync + 'static,
    {
        let service = self
            .client
            .service_builder()
            .description("esrc-cqrs command dispatcher")
            .start(&self.service_name, CMD_SERVICE_VERSION)
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        let group = service.group(&self.service_name);

        // Build one endpoint per handler and spawn a task for each.
        let mut tasks = tokio::task::JoinSet::new();

        for handler in handlers {
            let handler = Arc::clone(handler);
            let mut store = store.clone();

            let mut endpoint = group
                .endpoint(handler.name())
                .await
                .map_err(|e| Error::Internal(e.into()))?;

            tasks.spawn(async move {
                while let Some(request) = endpoint.next().await {
                    let payload = request.message.payload.as_ref();
                    match handler.handle_erased(&mut store, payload).await {
                        Ok(reply) => {
                            let _ = request.respond(Ok(reply.into())).await;
                        },
                        Err(e) => {
                            // Send an error reply so the caller is not left waiting.
                            let msg = format!("{e}");
                            let _ = request
                                .respond(Err(async_nats::service::error::Error {
                                    code: 500,
                                    status: msg,
                                }))
                                .await;
                        },
                    }
                }
                error::Result::Ok(())
            });
        }

        // Wait for all endpoint tasks; return the first error encountered.
        while let Some(result) = tasks.join_next().await {
            result.map_err(|e| Error::Internal(e.into()))??;
        }

        Ok(())
    }
}

/// Build the full NATS subject for a command endpoint.
///
/// Pattern: `<service_name>.<handler_name>`
pub fn command_subject(service_name: &str, handler_name: &str) -> String {
    format!("{service_name}.{handler_name}")
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/projector_runner.rs">
use tracing::instrument;

use esrc::error;
use esrc::nats::NatsStore;

use crate::projector::ProjectorHandler;

/// NATS projector runner.
///
/// Wraps a [`ProjectorHandler`] and drives it against a [`NatsStore`].
/// The runner simply delegates to the handler's `run` method, which internally
/// uses the durable JetStream consumer subscribe path already present in the
/// `NatsStore` via `durable_observe`.
///
/// Each runner should be started in its own Tokio task so that projectors
/// run concurrently and independently.
pub struct NatsProjectorRunner<H> {
    handler: H,
}

impl<H> NatsProjectorRunner<H>
where
    H: ProjectorHandler<NatsStore> + Send + 'static,
{
    /// Create a new runner wrapping the given handler.
    pub fn new(handler: H) -> Self {
        Self { handler }
    }

    /// Run the projector against the given store.
    ///
    /// This drives the handler until it returns an error. The caller is
    /// responsible for respawning or handling the error as appropriate.
    #[instrument(skip_all, level = "debug")]
    pub async fn run(&self, store: &NatsStore) -> error::Result<()> {
        self.handler.run(store).await
    }
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/aggregate_command_handler.rs">
use std::marker::PhantomData;

use esrc::aggregate::{Aggregate, Root};
use esrc::error::{self, Error};
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
    /// An optional message (e.g., error description on failure).
    pub message: Option<String>,
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
            serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;

        let root: Root<A> = store.read(envelope.id).await?;
        let root = store.try_write(root, envelope.command, None).await?;

        let reply = CommandReply {
            id: Root::id(&root),
            success: true,
            message: None,
        };
        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
    }
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/aggregate_projector_handler.rs">
use std::pin::pin;

use esrc::error::{self, Error};
use esrc::event::future::IntoSendFuture;
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use futures::StreamExt;
use tracing::instrument;

use crate::projector::ProjectorHandler;

/// A [`ProjectorHandler`] backed by a NATS JetStream durable consumer.
///
/// Uses `NatsStore::durable_observe` to subscribe to events with a named
/// durable consumer, ensuring the projector resumes from its last position
/// across process restarts.
///
/// `P` is the [`Project`] implementation to drive.
pub struct DurableProjectorHandler<P> {
    /// The durable consumer name; also used as the projector's routing name.
    durable_name: &'static str,
    projector: P,
}

impl<P> DurableProjectorHandler<P>
where
    P: Project + Clone + Send + Sync + 'static,
{
    /// Create a new handler with the given durable name and projector.
    ///
    /// The `durable_name` becomes the NATS durable consumer name and must be
    /// unique across all projectors in the application.
    pub fn new(durable_name: &'static str, projector: P) -> Self {
        Self {
            durable_name,
            projector,
        }
    }
}

impl<P> ProjectorHandler<NatsStore> for DurableProjectorHandler<P>
where
    P: Project + Clone + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        self.durable_name
    }

    #[instrument(skip_all, level = "debug")]
    async fn run<'a>(&'a self, store: &'a NatsStore) -> error::Result<()> {
        store
            .durable_observe(self.projector.clone(), self.durable_name)
            .await
    }
}
</FILE_NEW>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
 pub use command_dispatcher::NatsCommandDispatcher;
 pub use projector_runner::NatsProjectorRunner;
+pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
+pub use aggregate_projector_handler::DurableProjectorHandler;
+
+mod aggregate_command_handler;
+mod aggregate_projector_handler;
</FILE_PATCH>

<FILE_PATCH file_path="Cargo.toml">
@@
 [workspace]
-members = [".", "./derive", "./crates/opentelemetry-nats"]
+members = [".", "./derive", "./crates/opentelemetry-nats", "./crates/esrc-cqrs"]
 resolver = "2"
</FILE_PATCH>

</FILE_CHANGES>

Here is a summary of what was created:

**`crates/esrc-cqrs/`** - new extension crate with:

- `src/command.rs` - `CommandHandler<S>` trait: takes raw bytes, returns raw bytes, named for routing
- `src/projector.rs` - `ProjectorHandler<S>` trait: named durable handler, runs indefinitely
- `src/registry.rs` - `CqrsRegistry<S>`: stores handlers behind object-safe erased wrappers, drives projectors via `JoinSet`, exposes `run_projectors()`
- `src/nats/command_dispatcher.rs` - uses **NATS Services API** (request/reply service groups), one endpoint per handler; correct choice because commands are synchronous, targeted, and do not need durability
- `src/nats/projector_runner.rs` - thin wrapper delegating to `durable_observe`; correct choice because projectors need durable at-least-once delivery with resumable position
- `src/nats/aggregate_command_handler.rs` - `AggregateCommandHandler<A>`: loads aggregate via `ReplayOneExt::read`, writes via `PublishExt::try_write`, wraps command in `CommandEnvelope { id, command }`
- `src/nats/aggregate_projector_handler.rs` - `DurableProjectorHandler<P>`: wraps any `Project` impl with a durable name, delegates to `durable_observe`

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs - Add CQRS extension crate with command and event handler registry with NATS implementation"
</suggested_git_command>

