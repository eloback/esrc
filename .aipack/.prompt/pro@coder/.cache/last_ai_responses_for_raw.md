====
>   Info: Duration: 36s | ~$0.08 | Prompt Tokens: 14,451 (cached: 0, cache_creation: 0) | Completion Tokens: 2,443 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll review the documentation and exports across the crate, focusing on the query-related parts that are new and under-documented, as well as ensuring the module-level docs and `lib.rs` exports are consistent.

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/src/lib.rs">
@@
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
+//!
+//! # Query Handlers
+//!
+//! A query handler receives a typed query request, loads the required data
+//! (e.g., replaying aggregate state or reading a read model), and returns a
+//! serialized response. Queries are read-only: the store reference is shared
+//! rather than exclusive, so no events are written during query processing.
+//! Handlers are registered by name and dispatched by subject.
+//!
+//! # NATS Backend
+//!
+//! When the `nats` feature is enabled, concrete implementations are provided
+//! for all three handler kinds:
+//!
+//! - [`nats::NatsCommandDispatcher`] drives command handlers over core NATS
+//!   request/reply service groups.
+//! - [`nats::NatsQueryDispatcher`] drives query handlers over the same
+//!   mechanism, using a shared store reference.
+//! - [`nats::NatsProjectorRunner`] drives projectors via JetStream durable
+//!   pull consumers so they resume across restarts.
</FILE_PATCH>

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
+//!
+//! # Query Dispatcher
+//!
+//! Queries use NATS **core request/reply**, the same transport as commands, but
+//! with a shared (non-exclusive) store reference because queries are read-only.
+//! The dispatcher creates a service group and listens on subjects of the form
+//! `<service_name>.<handler_name>`. Each incoming request is dispatched to the
+//! matching [`QueryHandler`], and the reply is sent back to the caller.
+//!
+//! This is the correct transport choice for queries because:
+//! * Queries are point-in-time reads that expect an immediate response.
+//! * Sharing the store across handlers avoids unnecessary cloning of connections.
+//! * Core NATS request/reply naturally load-balances across service instances.
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/query.rs">
@@
 /// A handler for a single query type.
 ///
 /// Implementors receive a raw byte payload (the serialized query request),
 /// perform all necessary data retrieval (e.g., loading a read model or
 /// replaying aggregate state), and return a serialized response payload.
 ///
 /// The generic parameter `S` is the event store type (e.g., `NatsStore`).
 ///
 /// Queries are read-only by convention: a `QueryHandler` should never write
 /// events or mutate aggregate state. The store reference is therefore shared
 /// (`&S`) rather than exclusive (`&mut S`).
+///
+/// # Usage
+///
+/// For the common case of loading a single aggregate and projecting its state,
+/// use [`crate::nats::AggregateQueryHandler`] rather than implementing this
+/// trait directly.
+///
+/// For custom queries (e.g., cross-aggregate reads or external data sources),
+/// implement this trait directly and register the handler with
+/// [`crate::CqrsRegistry::register_query`].
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/query_dispatcher.rs">
@@
 /// Version string for the NATS query service group.
 pub const QUERY_SERVICE_VERSION: &str = "0.1.0";
@@
 /// NATS query dispatcher.
 ///
 /// Registers all query handlers as endpoints on a single NATS service,
 /// using core NATS request/reply. Each handler name becomes one endpoint
 /// within the service group named `<service_name>`.
 ///
 /// The store reference is shared (`&S`) across all query handlers because
 /// queries are read-only by convention.
+///
+/// # Subject Pattern
+///
+/// Subjects follow the pattern `<service_name>.<handler_name>`, where
+/// `handler_name` is the value returned by [`crate::query::QueryHandler::name`].
+/// Use [`query_subject`] to build the subject string for a given handler.
+///
+/// # Reply Shape
+///
+/// Each endpoint returns a serialized [`crate::nats::QueryReply`]. On success,
+/// `success` is `true` and `data` contains the handler's response as a JSON
+/// value. On failure, `success` is `false` and `error` is set to a
+/// [`crate::Error`] describing the problem.
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/aggregate_query_handler.rs">
@@
 /// A generic [`QueryHandler`] implementation for NATS-backed aggregates.
 ///
 /// This handler:
 /// 1. Deserializes the incoming payload as a [`QueryEnvelope`].
 /// 2. Loads the aggregate using [`ReplayOneExt::read`].
 /// 3. Applies the user-supplied projection function to produce the response.
 /// 4. Returns a serialized [`QueryReply`] containing the response as JSON.
 ///
 /// `A` is the aggregate type. `A::Event` must implement `DeserializeVersion`.
 /// `R` is the response / read-model type and must implement `Serialize`.
 ///
 /// # Example
 ///
 /// ```rust,ignore
 /// let handler = AggregateQueryHandler::<MyAggregate, MyReadModel>::new(
 ///     "MyAggregate.GetState",
 ///     |root| MyReadModel::from(Root::state(root)),
 /// );
 /// ```
+///
+/// # Extending Queries
+///
+/// If a query requires parameters beyond the aggregate ID, implement
+/// [`crate::query::QueryHandler`] directly and deserialize a custom request
+/// envelope inside `handle`. Register the custom handler with
+/// [`crate::CqrsRegistry::register_query`] as usual.
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/registry.rs">
@@
 /// A registry that holds command handlers and event projectors.
 ///
 /// `S` is the event store type shared across all handlers. The store must be
 /// `Clone` so that each command handler and projector can hold its own handle.
+///
+/// # Building a Registry
+///
+/// Use the builder-style `register_*` methods to attach handlers, then call
+/// [`CqrsRegistry::run_projectors`] to start projectors as background tasks.
+/// Pass the registry's handler slices to the appropriate dispatcher
+/// (e.g., [`crate::nats::NatsCommandDispatcher`] or
+/// [`crate::nats::NatsQueryDispatcher`]) to begin serving requests.
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git commit -a -m "^ esrc-cqrs - Improve crate, module, and query-related documentation"
</suggested_git_command>

