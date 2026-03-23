# Context Files Descriptions (Sent to AI for selection)

- src/event/future.rs
    - Summary: Defines a helper trait for converting a `Future` into a `Send` future, working around a compiler issue where `Send` cannot be inferred for an inner future.
    - When To Use: Include this file when working with async/event code that needs to guarantee a future is `Send`, especially when compiler inference fails for nested futures.
    - Types: IntoSendFuture
    - Functions: into_send_future

- src/event/truncate.rs
    - Summary: Defines the Truncate trait for deleting old messages from an event stream up to a given sequence number.
    - When To Use: Use when you need the event-stream truncation API or want to implement/support pruning old events, typically alongside snapshotting to limit stream size.
    - Types: Truncate
    - Functions: truncate

- src/nats/convert.rs
    - Summary: Implements conversion from async_nats JetStream error types into the crate's unified Error type, including a special mapping for wrong last sequence publish errors to Conflict.
    - When To Use: Include this file when working on NATS/JetStream error handling, especially error conversion, mapping external NATS errors into the crate's Error type, or diagnosing publish/stream/consumer-related failures.

- src/kurrent/convert.rs
    - Summary: Implements conversion from kurrentdb errors into the crate's Error type, mapping optimistic concurrency conflicts to Conflict and all other errors to Internal.
    - When To Use: Include this file when working on error translation between kurrentdb and this crate, especially for handling expected-version conflicts.
    - Types: Error

- src/kurrent/subject.rs
    - Summary: Defines the KurrentSubject enum for representing event subjects (wildcard, event name, or aggregate name with UUID) and provides parsing/string conversion helpers.
    - When To Use: Include this file when working with subject parsing, subject string formatting, or logic that distinguishes wildcard, event, and aggregate subjects in the Kurrent domain.
    - Types: KurrentSubject
    - Functions: try_from_str, into_string

- src/kurrent.rs
    - Summary: Defines the KurrentStore wrapper around a kurrentdb Client for use as an event store implementation, and exposes related modules for event conversion and envelopes.
    - When To Use: Include this file when you need the KurrentStore type, its construction, or to understand the KurrentDB-backed event store module structure.
    - Types: KurrentStore
    - Functions: KurrentStore::try_new

- src/kurrent/header.rs
    - Summary: Defines constant header keys used for Kurrent/Esrc metadata, including version and event type headers.
    - When To Use: Include this file when working with code that reads, writes, or matches the custom Esrc/Kurrent HTTP or event metadata headers.

- src/version.rs
    - Summary: Defines version-aware serde extension traits for serializing and deserializing types with an associated version, and re-exports derive helpers when the derive feature is enabled.
    - When To Use: Include this file when working with versioned serialization/deserialization logic, serde integration, or when needing the `SerializeVersion`/`DeserializeVersion` traits (or their derive macros under the derive feature).
    - Types: DeserializeVersion, SerializeVersion

- src/event/subscribe.rs
    - Summary: Defines async subscription traits for event streams and a helper extension to observe subscribed events by projecting them with a Project implementation.
    - When To Use: Include this file when working with event subscription APIs, consuming new events from streams, or projecting incoming events into application state via SubscribeExt::observe.
    - Types: Subscribe, SubscribeExt
    - Functions: subscribe, observe

- src/event/replay.rs
    - Summary: Defines replay and replay extension traits for event streams, including helpers to rebuild projections and materialize aggregates from replayed events.
    - When To Use: Use this file when you need to replay events from an event store, rebuild a projector, or load/update an aggregate root from historical events.
    - Types: Replay, ReplayOne, ReplayExt, ReplayOneExt

- src/nats/subject.rs
    - Summary: Defines NATS subject variants and helpers for parsing and formatting subjects with a required prefix, event name, and optional aggregate UUID.
    - When To Use: Use when working with NATS subject routing in this crate, especially to parse incoming subject strings or build outgoing subject strings from domain values.
    - Types: NatsSubject
    - Functions: try_from_str, into_string

- src/event/publish.rs
    - Summary: Defines the event publishing traits for writing events to an event stream, including extension helpers that publish aggregate events and commands with optimistic concurrency support.
    - When To Use: Use this file when you need the API for publishing events, writing aggregate events back to the event store, or converting commands into events via aggregate helper methods.
    - Types: Publish, PublishExt
    - Functions: write, try_write

- src/kurrent/event.rs
    - Summary: Implements KurrentStore event publishing, replaying a single aggregate stream, and subscribing to event groups via KurrentDB persistent subscriptions, including durable subscribe/observe helpers.
    - When To Use: Use this file when working with KurrentDB-backed event store operations such as publishing events, reading aggregate history, creating persistent subscriptions, or running durable projector loops.
    - Functions: publish, publish_without_occ, replay_one, subscribe, durable_subscribe, durable_observe

- src/envelope.rs
    - Summary: Defines the core Envelope abstraction for event-store records and a helper trait for converting envelopes into event types.
    - When To Use: Include this file when you need the event envelope interface, envelope metadata access, or automatic/manual deserialization from stored event data into domain events.
    - Types: Envelope, TryFromEnvelope

- src/kurrent/envelope.rs
    - Summary: Defines KurrentEnvelope, an adapter that converts a kurrentdb ResolvedEvent into the crate’s Envelope trait implementation by extracting aggregate identity, sequence, timestamp, and version metadata, and providing version-aware event deserialization.
    - When To Use: Include this file when working with KurrentDB event ingestion, envelope conversion, or deserializing events from resolved KurrentDB records using the stream subject and version metadata.
    - Types: KurrentEnvelope
    - Functions: KurrentEnvelope::try_from_message

- src/nats/header.rs
    - Summary: Defines NATS header key constants and a helper for reading header values from an async_nats message.
    - When To Use: Use this file when working with NATS message headers, especially to access standardized event/version metadata or to extract header strings from messages.
    - Types: VERSION_KEY, EVENT_TYPE, METADATA_PREFIX
    - Functions: get

- src/project.rs
    - Summary: Defines the projection API for event-sourced processing, including a Context wrapper around deserialized envelope contents and the Project trait used to handle events and build read models or trigger side effects.
    - When To Use: Include this file when you need to understand or implement projection logic, access event envelope metadata within a projection, or work with the Project trait and Context wrapper.
    - Types: Context, Project
    - Functions: Context::try_with_envelope, Context::id, Context::sequence, Context::timestamp, Context::get_metadata, Context::into_inner

- src/nats/envelope.rs
    - Summary: Defines `NatsEnvelope`, an envelope wrapper for NATS JetStream messages that extracts aggregate metadata from the subject and headers, supports event deserialization by version, propagates tracing context, and acks messages on drop/use.
    - When To Use: Include this file when working with NATS JetStream event ingestion, message-to-envelope conversion, extracting event metadata from NATS headers/subjects, or deserializing versioned events from NATS payloads.
    - Types: NatsEnvelope
    - Functions: NatsEnvelope::try_from_message, NatsEnvelope::attach_span_context, NatsEnvelope::ack

- src/aggregate.rs
    - Summary: Defines the core Aggregate trait for event-sourced domain objects and the Root wrapper that materializes an aggregate with stream identity and sequence tracking.
    - When To Use: Include when you need the aggregate abstraction, root aggregate state management, event application/validation, or to understand how commands are processed into events in this event-sourcing library.
    - Types: Aggregate, Root
    - Functions: Root::with_aggregate, Root::id, Root::last_sequence, Root::into_inner, Root::new, Root::try_apply

- src/view.rs
    - Summary: Defines the `View` trait, a lightweight reactive read model built from an event stream for query/projection purposes.
    - When To Use: Include this file when working with event-sourced read models, projections, or any code that implements or uses the `View` trait to replay/apply events.
    - Types: View

- Cargo.toml
    - Summary: Workspace and root package manifest for the esrc Rust project, defining shared dependencies, feature flags, and member crates/examples.
    - When To Use: Use this file when you need to understand the project structure, enabled feature combinations, dependency versions, workspace members, or package metadata for builds and integration context.

- crates/esrc-cqrs/src/nats/projector_runner.rs
    - Summary: Defines a NATS-specific projector runner that wraps a ProjectorHandler and executes it against a NatsStore, typically in its own Tokio task for concurrent projector execution.
    - When To Use: Use when working with NATS-backed CQRS projector execution, especially to start or invoke a projector handler against a NatsStore.
    - Types: NatsProjectorRunner<H>
    - Functions: new(handler: H) -> Self, run(&self, store: &NatsStore) -> error::Result<()>

- crates/esrc-cqrs/src/nats/query_dispatcher.rs
    - Summary: Implements a NATS-based query dispatcher that registers each query handler as a service endpoint and forwards request/reply queries to erased handlers. Also provides a helper to build query subjects.
    - When To Use: Use this file when working on CQRS query transport over NATS, especially for starting the query service, wiring handlers into endpoints, or constructing query subject names.
    - Types: QueryEnvelope, QueryReply, NatsQueryDispatcher
    - Functions: query_subject

- crates/esrc-cqrs/src/projector.rs
    - Summary: Defines the ProjectorHandler trait used to run event projector subscriptions against an event store, including durable naming and execution logic.
    - When To Use: Include this file when working with CQRS projectors, subscription handlers, or code that needs to define/run durable or transient projector tasks over an event store.
    - Types: ProjectorHandler

- crates/esrc-cqrs/src/query.rs
    - Summary: Defines the QueryHandler trait for handling read-only query messages by name, using a shared event store reference and returning serialized response bytes.
    - When To Use: Include this file when working on CQRS query handling, query routing by handler name, or implementing/consuming read-only handlers that deserialize request payloads and serialize replies.
    - Types: QueryHandler

- crates/esrc-cqrs/src/registry.rs
    - Summary: Defines a CQRS registry that stores command, projector, and query handlers, provides registration/accessors, and can spawn all projectors as Tokio background tasks. Also includes object-safe erased wrapper traits for heterogeneous command, projector, and query handlers.
    - When To Use: Include this file when you need to understand or use the CQRS handler registry, register command/projector/query implementations, access registered handlers or the shared store, or run projectors concurrently.
    - Types: CqrsRegistry<S>, ErasedCommandHandler<S>, ErasedProjectorHandler<S>, ErasedQueryHandler<S>
    - Functions: CqrsRegistry::new, CqrsRegistry::register_command, CqrsRegistry::register_projector, CqrsRegistry::register_query, CqrsRegistry::command_handlers, CqrsRegistry::projector_handlers, CqrsRegistry::query_handlers, CqrsRegistry::store, CqrsRegistry::run_projectors

- src/event.rs
    - Summary: Defines the core event abstractions for the crate, including the Event and EventGroup traits, the Sequence wrapper for stream ordering, and re-exports for publish/replay/subscribe/truncate event-store operations.
    - When To Use: Use this file when you need the central event-sourcing API surface: event type definitions, stream sequence handling, grouping of event types, or to access the main publish/replay/subscribe/truncate traits and helpers.
    - Types: Sequence, Event, EventGroup
    - Functions: Sequence::new

- src/lib.rs
    - Summary: Crate root for the event-sourcing library, declaring core modules for aggregates, envelopes, errors, events, projections, and versioning, plus optional event store integrations.
    - When To Use: Use this file to understand the library’s overall module layout, available top-level re-exports, and which backend integrations are conditionally compiled.
    - Types: Aggregate, Envelope, Error, Event, EventGroup

- src/nats.rs
    - Summary: Defines the NatsStore event-store wrapper over NATS JetStream, including stream setup, optional mirror stream support, consumer creation helpers, and graceful shutdown tracking.
    - When To Use: Include this file when working with NATS/JetStream-backed event storage, stream or consumer initialization, mirror/read-side setup, or graceful task shutdown logic.
    - Types: NatsStore, GracefulShutdown
    - Functions: NatsStore::try_new, NatsStore::enable_mirror, NatsStore::get_task_tracker, NatsStore::wait_graceful_shutdown, NatsStore::update_durable_consumer_option

- src/nats/event.rs
    - Summary: Implements NATS-backed event store operations for publishing, replaying, subscribing, durable observation, and truncation of events, including header metadata handling and envelope conversion.
    - When To Use: Use when you need the NATS event store behavior, especially for publishing events with headers, replaying or subscribing to event streams, durable projection consumption, or truncating aggregate streams.
    - Types: NatsStore, NatsEnvelope
    - Functions: NatsStore::durable_observe, Replay::replay, ReplayOne::replay_one, Subscribe::subscribe, Truncate::truncate

- crates/esrc-cqrs/.devenv/bootstrap/bootstrapLib.nix
    - Summary: Shared Nix library for devenv evaluation and bootstrap logic, including overlay resolution, module import handling, profile expansion/priority application, cross-system evaluation, and a lightweight input evaluator.
    - When To Use: Include this file when you need to understand or modify how devenv configuration is evaluated, how profiles and overlays are resolved, or how input/project devenv modules are loaded and built for one or more systems.
    - Functions: getOverlays, mkDevenvForSystem, mkDevenvForInput

- crates/esrc-cqrs/.devenv/bootstrap/default.nix
    - Summary: Nix bootstrap entrypoint that resolves locked inputs and delegates to the bootstrap library to construct the devenv configuration for the current system.
    - When To Use: Include this file when you need the Nix bootstrap flow for this project, especially to understand how the devenv environment is assembled or to trace the system-specific bootstrap entrypoint.

- crates/esrc-cqrs/.devenv/bootstrap/resolve-lock.nix
    - Summary: Nix bootstrap helper that reads devenv.lock, resolves flake/path inputs (including relative paths), and constructs per-node flake/devenv evaluation data.
    - When To Use: Use when you need to understand or modify how the project bootstraps devenv/flake inputs from the lockfile, especially lock resolution and lazy evaluation behavior.

- crates/esrc-cqrs/Cargo.toml
    - Summary: Cargo manifest for the esrc-cqrs crate, defining package metadata, feature flags, and dependencies for CQRS command/event handler registry support.
    - When To Use: Include this file when you need to understand the crate's build configuration, enabled features, or dependency relationships for the CQRS registry integration.

- crates/esrc-cqrs/.gitignore
    - Summary: Git ignore rules for the esrc-cqrs crate, excluding build artifacts, backup Rust files, and local development tool files.
    - When To Use: Include when you need to understand which generated, local, or environment-specific files are intentionally excluded from version control for this crate.

- crates/esrc-cqrs/examples/cafe/domain.rs
    - Summary: Defines the cafe order domain model for a CQRS example, including the Order aggregate, commands, events, errors, and a read-model projection.
    - When To Use: Include this file when you need the domain logic for the cafe example, especially the order aggregate behavior, event types, command handling, or state projection.
    - Types: OrderStatus, Order, OrderCommand, OrderEvent, OrderError, OrderState
    - Functions: Order::process, Order::apply, View::apply

- crates/esrc-cqrs/examples/cafe/projector.rs
    - Summary: Defines an OrderProjector that handles OrderEvent values and prints order activity to stdout.
    - When To Use: Include this file when you need the example CQRS projector implementation for cafe order events, especially to understand how OrderEvent is projected or logged.
    - Types: OrderProjector

- crates/esrc-cqrs/src/nats/query/mod.rs
    - Summary: Module entry point for NATS query handlers, re-exporting live-view and memory-view query implementations.
    - When To Use: Use this file when you need the available NATS query abstractions or want to import the query handler types from a single module path.
    - Types: LiveViewQuery, MemoryView, MemoryViewQuery

- crates/esrc-cqrs/src/nats/query/live_view_query.rs
    - Summary: Implements a NATS query handler that rebuilds a view by replaying an aggregate's full event history on each request, then returns a projected serializable read model.
    - When To Use: Use when you need to understand or modify live query handling for replay-on-demand read models over NATS, especially the QueryHandler implementation, replay logic, or projection behavior.
    - Types: LiveViewQuery<V, R>
    - Functions: LiveViewQuery::new, LiveViewQuery::new_for_serializable_view

- crates/esrc-cqrs/src/nats/client/mod.rs
    - Summary: Module entry point for the NATS CQRS client, exposing the high-level ergonomic CQRS client API.
    - When To Use: Use this file when you need the public CQRS client type for NATS-based command/query dispatch or want to trace what the client module exports.
    - Types: CqrsClient

- crates/esrc-cqrs/src/nats/durable_projector_handler.rs
    - Summary: Defines a projector handler that uses a NATS JetStream durable consumer so event processing can resume from the last checkpoint after restarts.
    - When To Use: Include this file when working on NATS-backed projector execution, durable consumer setup, or understanding how projector handlers are wired to resume event streams.
    - Types: DurableProjectorHandler<P>
    - Functions: new(durable_name: &'static str, projector: P) -> Self, name(&self) -> &'static str, run<'a>(&'a self, store: &'a NatsStore) -> error::Result<()>

- crates/esrc-cqrs/.devenv/nixpkgs-config-c7c9559ef5bdea25.nix
    - Summary: Nix configuration snippet that defines the allowUnfreePredicate logic for nixpkgs, controlling which unfree packages are permitted.
    - When To Use: Include when working on Nix/devenv configuration for this crate, especially if you need to understand or modify how unfree packages are allowed or filtered.

- crates/esrc-cqrs/src/nats/command/aggregate_command_handler.rs
    - Summary: Implements a generic NATS-backed command handler for aggregates. It defines the command envelope format, loads an aggregate from the NATS store, applies a command, and serializes a standardized reply while converting framework errors into CQRS-friendly errors.
    - When To Use: Use this file when working on NATS command handling for aggregates, command envelope serialization/deserialization, aggregate command execution flow, or error conversion between esrc and cqrs error types.
    - Types: CommandEnvelope<C>, AggregateCommandHandler<A>
    - Functions: AggregateCommandHandler::new, convert_esrc_error<A>

- crates/esrc-cqrs/tests/integration_nats.rs
    - Summary: Integration tests for esrc-cqrs against a live NATS JetStream server, covering command dispatch, durable event storage, projector behavior, error propagation, malformed payload handling, query handling, and registry accessors.
    - When To Use: Include this file when you need to understand or verify the NATS/JetStream integration behavior of esrc-cqrs, especially request/reply command and query handling, projector execution, durability, or end-to-end test setup.
    - Types: CounterState, Counter, CounterCommand, CounterEvent, CounterError, RecordingProjector, ProjectorError
    - Functions: test_command_request_response_success, test_command_error_does_not_break_dispatcher, test_projector_receives_events, test_projector_acks_messages_no_redelivery, test_projector_error_propagates, test_multiple_commands_same_aggregate_occ, test_malformed_payload_returns_error, test_registry_accessors, test_query_returns_aggregate_state, test_query_default_state_for_new_aggregate, test_query_malformed_payload_returns_error, test_registry_query_handlers_accessor

- crates/esrc-cqrs/src/nats/query/memory_view_query.rs
    - Summary: Implements an in-memory CQRS projection store and a NATS query handler that reads projected views from shared memory and serializes query replies.
    - When To Use: Include this file when working on NATS-based query handling, in-memory projections/read models, or code that needs to project events into shared views and answer queries from those views.
    - Types: MemoryView<V>, MemoryViewQuery<V, R>
    - Functions: MemoryView::new, MemoryViewQuery::new, MemoryViewQuery::new_for_serializable_view

- crates/esrc-cqrs/src/error.rs
    - Summary: Defines the serializable CQRS error type used for transport over NATS, plus conversion from the core esrc error type and helpers to recover typed external errors.
    - When To Use: Use when handling CQRS command errors, serializing/deserializing errors across the transport boundary, or converting between esrc::error::Error and the CQRS-specific serializable Error type.
    - Types: Error, Result
    - Functions: from_esrc_error

- crates/esrc-cqrs/src/nats/command_dispatcher.rs
    - Summary: Implements a NATS-based command dispatcher that registers erased command handlers as request/reply endpoints on a NATS service and forwards requests to them, plus a helper for building command subjects.
    - When To Use: Include this file when you need to understand or modify how CQRS command handlers are exposed over NATS, how requests are dispatched to handlers, or how command subject names are constructed.
    - Types: CommandReply, NatsCommandDispatcher
    - Functions: NatsCommandDispatcher::new, NatsCommandDispatcher::run, command_subject

- src/error.rs
    - Summary: Defines the crate’s common event-sourcing error type and a Result alias for operations that can fail with these errors.
    - When To Use: Include this file when handling, matching, or propagating the library’s standardized errors, or when using the event-sourcing Result alias.
    - Types: Error, Result

- crates/esrc-cqrs/src/nats/client/cqrs_client.rs
    - Summary: Defines a high-level CQRS client wrapper around async_nats::Client that builds command/query envelopes, sends NATS requests, and deserializes replies. It also provides convenience dispatch methods that convert CQRS reply success/error handling into Result-based APIs.
    - When To Use: Include this file when working with CQRS over NATS, especially if you need the client-side helper for sending commands or queries, constructing subjects/envelopes, or handling reply deserialization and error conversion.
    - Types: CqrsClient
    - Functions: new, inner, send_command, dispatch_command, send_query, dispatch_query

- crates/esrc-cqrs/examples/cafe/main.rs
    - Summary: Executable cafe example showing how to wire up esrc-cqrs with NATS/JetStream, including command dispatching, query handling, and a durable projector for an order aggregate.
    - When To Use: Include this file when you need an end-to-end usage example of the CQRS setup, dispatcher wiring, projector execution, and NATS subject naming for the cafe domain.
    - Functions: main

- crates/esrc-cqrs/src/command.rs
    - Summary: Defines the CQRS command-handling traits used to route and process incoming command messages against an event store, including both raw command handlers and NATS service-level command handlers.
    - When To Use: Include this file when working with command dispatch, implementing new command handlers, or understanding how raw or typed command payloads are handled and replied to in the CQRS layer.
    - Types: CommandHandler, NatsServiceCommandHandler

- crates/esrc-cqrs/src/lib.rs
    - Summary: Top-level library module for the `esrc-cqrs` crate. It documents the CQRS extension, exposes modules for command handling, queries, projectors, the main registry, and optional NATS integrations, and re-exports the primary CQRS traits and registry type.
    - When To Use: Include this file when you need the crate’s public API surface, an overview of CQRS support, or to find where command handlers, query handlers, projector handlers, and the registry are defined and re-exported.
    - Types: CommandHandler, Error, ProjectorHandler, CqrsRegistry, QueryHandler, CqrsClient

- crates/esrc-cqrs/src/nats/command/mod.rs
    - Summary: Module wiring for NATS command handling. It exposes the aggregate command handler module and service command handler adapter, and re-exports the main command-handling types.
    - When To Use: Use this file when you need the command-side NATS CQRS wiring or want access to the aggregate or service command handler types via the module re-exports.
    - Types: AggregateCommandHandler, CommandEnvelope, CommandReply, ServiceCommandHandler

- crates/esrc-cqrs/src/nats/command/service_command_handler.rs
    - Summary: Defines an adapter that wraps a NATS service command handler and exposes it through the generic CommandHandler interface by deserializing JSON payloads and delegating to the underlying handler.
    - When To Use: Use this file when you need to understand or include the bridge between NATS service command handlers and the generic command registry/dispatcher, especially for JSON command deserialization and endpoint name forwarding.
    - Types: ServiceCommandHandler<H, C>
    - Functions: ServiceCommandHandler::new

- crates/esrc-cqrs/src/nats/mod.rs
    - Summary: NATS CQRS integration module that wires together command dispatching over core NATS request/reply, query dispatching, and projector execution over JetStream durable pull consumers.
    - When To Use: Include this file when you need the NATS-backed CQRS entry points, especially to understand or import the dispatcher and projector runner types re-exported from this module.
    - Types: NatsCommandDispatcher, NatsProjectorRunner, NatsQueryDispatcher, QueryEnvelope, QueryReply, DurableProjectorHandler

