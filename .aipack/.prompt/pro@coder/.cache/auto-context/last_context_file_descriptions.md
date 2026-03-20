# Context Files Descriptions (Sent to AI for selection)

- derive/Cargo.toml
    - Summary: Cargo manifest for the `esrc-derive` procedural macro crate, defining package metadata, proc-macro library settings, and dependencies used by the derive macros.
    - When To Use: Include this file when you need to understand the build configuration, crate type, or dependency set for the `esrc-derive` proc-macro crate.

- derive/src/lib.rs
    - Summary: Proc-macro entry point for the derive crate. It defines shared ESRC derive attributes and exposes derive macros for Event, EventGroup, SerializeVersion, DeserializeVersion, and TryFromEnvelope by delegating to internal modules.
    - When To Use: Use this file when you need the public derive macros or want to understand the top-level macro definitions, supported attributes, and how the derive crate is wired together.
    - Types: EsrcAttributes
    - Functions: DeserializeVersion, Event, EventGroup, SerializeVersion, TryFromEnvelope

- derive/src/util.rs
    - Summary: Utility module that re-exports the `lifetime` and `variant` submodules for the derive crate.
    - When To Use: Include this file when you need the top-level utility module for the derive crate or want to access its `lifetime` and `variant` helpers.

- derive/src/util/lifetime.rs
    - Summary: Utility helpers for working with Rust lifetimes in derive macro generics, adding outlives bounds and converting them into where-clause predicates.
    - When To Use: Include this file when you need to understand or modify how the derive code augments Generics with lifetime parameters and lifetime outlives constraints.
    - Functions: add_supertype_bounds

- derive/src/util/variant.rs
    - Summary: Utility helpers for inspecting and filtering enum variants in derive macros, including collecting variants, extracting a variant's inner type, and conditionally ignoring variants based on parsed attributes.
    - When To Use: Use this file when working on derive macro logic that needs to process enum variants, filter them by attributes, or validate/extract the single inner field of newtype variants.
    - Functions: try_collect, try_into_inner_type

- derive/tests/fixtures/mod.rs
    - Summary: Test fixtures module that re-exports fixture submodules for derive-related tests.
    - When To Use: Include when working on derive test fixtures or when tests need access to the envelope, event, or version fixture modules.

- derive/tests/fixtures/version.rs
    - Summary: Test fixture providing a helper that constructs a unit deserializer for serde tests.
    - When To Use: Include when working on derive test fixtures or tests that need a simple serde unit deserializer implementation.
    - Functions: unit_deserializer

- examples/cafe/error.rs
    - Summary: Defines the Cafe example's tab-related error type used for tab state and payment validation.
    - When To Use: Include when working with the cafe example's error handling or any code that matches on or returns tab-related errors.
    - Types: TabError

- examples/cafe/tab.rs
    - Summary: Defines a cafe tab aggregate with commands, events, and state transitions for opening a tab, placing orders, marking items served, and closing the tab with payment validation.
    - When To Use: Use when you need the domain model and business rules for the cafe tab example, especially the command/event definitions or how the aggregate validates and applies tab lifecycle changes.
    - Types: Item, TabCommand, TabEvent, Tab

- src/error.rs
    - Summary: Defines the crate’s common event-sourcing error type and a Result alias for operations that can fail with these errors.
    - When To Use: Include this file when handling, matching, or propagating the library’s standardized errors, or when using the event-sourcing Result alias.
    - Types: Error, Result

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

- examples/cafe/tab/tests.rs
    - Summary: Unit tests for the cafe tab aggregate, covering opening a tab, placing orders, marking items served, and closing tabs with both success and error cases.
    - When To Use: Include this file when you need to understand or validate the expected behavior of the Tab domain logic, event transitions, and error handling in the cafe example.
    - Functions: open_tab, order_with_unopened_tab, order, serve_twice, serve, close_with_underpayment, close

- src/kurrent/convert.rs
    - Summary: Implements conversion from kurrentdb errors into the crate's Error type, mapping optimistic concurrency conflicts to Conflict and all other errors to Internal.
    - When To Use: Include this file when working on error translation between kurrentdb and this crate, especially for handling expected-version conflicts.
    - Types: Error

- derive/src/event.rs
    - Summary: Implements procedural macro helpers for deriving `Event` and `EventGroup` behavior from Rust types, including event name generation, enum variant type reporting, and skipping ignored event-group variants.
    - When To Use: Use when you need to understand or modify the derive macros that generate `::esrc::event::Event` and `::esrc::event::EventGroup` impls for structs/enums, especially around event naming rules, suffix stripping, and variant collection.
    - Types: EventMeta
    - Functions: derive_event, derive_event_group

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

- derive/tests/test_event.rs
    - Summary: Integration tests for the Event and EventGroup derive macros, verifying generated event names, suffix handling, custom naming, generic/lifetime support, and ignored enum variants.
    - When To Use: Include this file when working on or debugging the esrc_derive event macro behavior, especially name generation and EventGroup variant filtering.
    - Types: Test, TestEvent, TestGroup
    - Functions: event, event_suffix, event_keep_suffix, event_name, event_generics, event_group, event_group_ignore, event_group_lifetime

- derive/tests/test_version.rs
    - Summary: Integration tests for the derive macros that implement version-aware serialization/deserialization traits, including default versioning, explicit version attributes, lifetime-bearing types, and upgrading from a previous version.
    - When To Use: Include this file when you need to understand or verify how the version derive macros are expected to behave in tests, especially around DeserializeVersion/SerializeVersion generation and previous-version conversion support.
    - Types: TestEvent, TestEvent1, TestEvent2
    - Functions: deserialize_version, deserialize_version_lifetime, deserialize_version_previous, serialize_version, serialize_version_default

- src/lib.rs
    - Summary: Crate root for the event-sourcing library, declaring core modules for aggregates, envelopes, errors, events, projections, and versioning, plus optional event store integrations.
    - When To Use: Use this file to understand the library’s overall module layout, available top-level re-exports, and which backend integrations are conditionally compiled.
    - Types: Aggregate, Envelope, Error, Event, EventGroup

- derive/src/envelope.rs
    - Summary: Implements the procedural macro logic for deriving `TryFromEnvelope` on enums by matching envelope names to single-field variants and deserializing the payload into the variant field.
    - When To Use: Include this file when working on or debugging the derive macro that converts an `Envelope` into an enum via `TryFromEnvelope`, especially when inspecting variant validation, generated matching logic, or deserialization behavior.
    - Types: TryFromEnvelopeArgs
    - Functions: derive_try_from_envelope

- derive/src/version.rs
    - Summary: Implements procedural macro helpers for deriving versioned serialization and deserialization traits, including support for current and previous schema versions.
    - When To Use: Use when you need to understand or modify the derive logic that generates `SerializeVersion` and `DeserializeVersion` implementations, especially version selection and backward-compatibility behavior.
    - Types: SerdeMeta
    - Functions: derive_deserialize_version, derive_serialize_version

- derive/tests/fixtures/envelope.rs
    - Summary: Test fixture defining a minimal `EmptyEnvelope` type that implements the `Envelope` trait for derive-related tests.
    - When To Use: Use when tests need a simple mock envelope implementation with fixed/default values and version deserialization behavior.
    - Types: EmptyEnvelope
    - Functions: EmptyEnvelope::new

- derive/tests/fixtures/event.rs
    - Summary: Test fixture defining several example event types and their `Event`/`DeserializeVersion` implementations, including a lifetime-bearing event.
    - When To Use: Use when tests or examples need concrete event fixtures to validate deriving, event naming, or versioned deserialization behavior.
    - Types: FooEvent, BarEvent, LifetimeEvent<'a>

- derive/tests/test_envelope.rs
    - Summary: Test module for the TryFromEnvelope derive macro, verifying that envelope type names are correctly converted into enum variants, including ignored variants and lifetime-generic cases.
    - When To Use: Use when working on or reviewing the TryFromEnvelope derive macro, its code generation, or its behavior against envelope-to-enum conversion tests.
    - Functions: try_from_envelope, try_from_envelope_ignore, try_from_envelope_lifetime

- examples/zero_copy/main.rs
    - Summary: Example showing how to define a zero-copy event type and a simple projector that observes events from a NATS JetStream-backed event store.
    - When To Use: Include this file when you need an example of event-store usage with serde zero-copy deserialization, the Project trait implementation, or wiring up a NATS store and observer in an async main.
    - Types: ZeroCopyEvent, NamePrinter, NamePrinterError
    - Functions: main

- src/aggregate.rs
    - Summary: Defines the core Aggregate trait for event-sourced domain objects and the Root wrapper that materializes an aggregate with stream identity and sequence tracking.
    - When To Use: Include when you need the aggregate abstraction, root aggregate state management, event application/validation, or to understand how commands are processed into events in this event-sourcing library.
    - Types: Aggregate, Root
    - Functions: Root::with_aggregate, Root::id, Root::last_sequence, Root::into_inner, Root::new, Root::try_apply

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

- examples/cafe/table.rs
    - Summary: Defines ActiveTables, a project state manager that tracks open cafe tables by UUID and table number, with helper queries for active tables.
    - When To Use: Use when you need to understand or modify how table open/close events are stored, queried, or projected into shared state in the cafe example.
    - Types: ActiveTables
    - Functions: new, is_active, get_table_numbers, project

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

- src/event.rs
    - Summary: Defines the core event abstractions for the crate, including the Event and EventGroup traits, the Sequence wrapper for stream ordering, and re-exports for publish/replay/subscribe/truncate event-store operations.
    - When To Use: Use this file when you need the central event-sourcing API surface: event type definitions, stream sequence handling, grouping of event types, or to access the main publish/replay/subscribe/truncate traits and helpers.
    - Types: Sequence, Event, EventGroup
    - Functions: Sequence::new

- src/nats.rs
    - Summary: Defines the NatsStore event-store wrapper over NATS JetStream, including stream setup, optional mirror stream support, consumer creation helpers, and graceful shutdown tracking.
    - When To Use: Include this file when working with NATS/JetStream-backed event storage, stream or consumer initialization, mirror/read-side setup, or graceful task shutdown logic.
    - Types: NatsStore, GracefulShutdown
    - Functions: NatsStore::try_new, NatsStore::enable_mirror, NatsStore::get_task_tracker, NatsStore::wait_graceful_shutdown, NatsStore::update_durable_consumer_option

- crates/esrc-cqrs/src/command.rs
    - Summary: Defines the CQRS command-handling trait used to route and process incoming command messages against an event store.
    - When To Use: Include this file when working with command dispatch, implementing new command handlers, or understanding how raw command payloads are handled and replied to in the CQRS layer.
    - Types: CommandHandler

- crates/esrc-cqrs/src/lib.rs
    - Summary: Top-level library module for the `esrc-cqrs` crate. It documents the CQRS extension, exposes modules for command handling, projectors, and the main registry, and re-exports the primary CQRS traits and registry type.
    - When To Use: Include this file when you need the crate’s public API surface, an overview of CQRS support, or to find where command handlers, projector handlers, and the registry are defined and re-exported.
    - Types: CommandHandler, ProjectorHandler, CqrsRegistry

- crates/esrc-cqrs/src/nats/projector_runner.rs
    - Summary: Defines a NATS-specific projector runner that wraps a ProjectorHandler and executes it against a NatsStore, typically in its own Tokio task for concurrent projector execution.
    - When To Use: Use when working with NATS-backed CQRS projector execution, especially to start or invoke a projector handler against a NatsStore.
    - Types: NatsProjectorRunner<H>
    - Functions: new(handler: H) -> Self, run(&self, store: &NatsStore) -> error::Result<()>

- crates/esrc-cqrs/src/projector.rs
    - Summary: Defines the ProjectorHandler trait used to run event projector subscriptions against an event store, including durable naming and execution logic.
    - When To Use: Include this file when working with CQRS projectors, subscription handlers, or code that needs to define/run durable or transient projector tasks over an event store.
    - Types: ProjectorHandler

- crates/esrc-cqrs/src/nats/mod.rs
    - Summary: NATS CQRS integration module that wires together command dispatching over core NATS request/reply and projector execution over JetStream durable pull consumers.
    - When To Use: Include this file when you need the NATS-backed CQRS entry points, especially to understand or import the dispatcher and projector runner types re-exported from this module.
    - Types: AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler, NatsCommandDispatcher, NatsProjectorRunner

- src/nats/event.rs
    - Summary: Implements NATS-backed event store operations for publishing, replaying, subscribing, durable observation, and truncation of events, including header metadata handling and envelope conversion.
    - When To Use: Use when you need the NATS event store behavior, especially for publishing events with headers, replaying or subscribing to event streams, durable projection consumption, or truncating aggregate streams.
    - Types: NatsStore, NatsEnvelope
    - Functions: NatsStore::durable_observe, Replay::replay, ReplayOne::replay_one, Subscribe::subscribe, Truncate::truncate

- crates/esrc-cqrs/src/nats/aggregate_projector_handler.rs
    - Summary: Defines a durable NATS JetStream-backed projector handler that runs a `Project` via a named durable consumer, allowing resume from the last processed position after restarts.
    - When To Use: Include this file when working with NATS-based CQRS projection handlers, durable consumer setup, or the `ProjectorHandler` implementation for resuming projectors from persisted stream positions.
    - Types: DurableProjectorHandler<P>
    - Functions: DurableProjectorHandler::new(durable_name: &'static str, projector: P) -> Self

- crates/esrc-cqrs/src/nats/aggregate_command_handler.rs
    - Summary: Defines a generic NATS-backed aggregate command handler plus request/reply envelopes for routing, deserializing, applying, and responding to aggregate commands.
    - When To Use: Use this file when you need to handle commands sent over NATS for an event-sourced aggregate, including loading the aggregate, applying a command, and returning a serialized success reply.
    - Types: CommandEnvelope<C>, CommandReply, AggregateCommandHandler<A>
    - Functions: AggregateCommandHandler::new, CommandHandler<NatsStore>::name, CommandHandler<NatsStore>::handle

- crates/esrc-cqrs/src/nats/command_dispatcher.rs
    - Summary: Implements a NATS-based command dispatcher that registers erased command handlers as request/reply endpoints on a NATS service and forwards requests to them, plus a helper for building command subjects.
    - When To Use: Include this file when you need to understand or modify how CQRS command handlers are exposed over NATS, how requests are dispatched to handlers, or how command subject names are constructed.
    - Types: NatsCommandDispatcher
    - Functions: NatsCommandDispatcher::new, NatsCommandDispatcher::run, command_subject

- crates/esrc-cqrs/src/registry.rs
    - Summary: Defines a CQRS registry that stores command handlers and projector handlers, provides registration/accessors, and can spawn all projectors as Tokio background tasks. Also includes object-safe erased wrapper traits for heterogeneous command and projector handlers.
    - When To Use: Include this file when you need to understand or use the CQRS handler registry, register command/projector implementations, access registered handlers or the shared store, or run projectors concurrently.
    - Types: CqrsRegistry<S>, ErasedCommandHandler<S>, ErasedProjectorHandler<S>
    - Functions: CqrsRegistry::new, CqrsRegistry::register_command, CqrsRegistry::register_projector, CqrsRegistry::command_handlers, CqrsRegistry::projector_handlers, CqrsRegistry::store, CqrsRegistry::run_projectors

- examples/cafe/projector.rs
    - Summary: Defines `OrderProjector`, a `Project` implementation that handles `OrderEvent`s and prints order activity to stdout.
    - When To Use: Use this file when you need the projection logic for cafe order events, especially to understand how order events are rendered/logged during event handling.
    - Types: OrderProjector

- Cargo.toml
    - Summary: Workspace and root package manifest for the esrc Rust project, defining shared dependencies, feature flags, and member crates/examples.
    - When To Use: Use this file when you need to understand the project structure, enabled feature combinations, dependency versions, workspace members, or package metadata for builds and integration context.

- examples/cafe/main.rs
    - Summary: Entry point for the cafe CQRS example using NATS/JetStream. It wires together the NATS store, command dispatcher, and durable projector, then sends sample order commands to demonstrate the flow.
    - When To Use: Include this file when you need the runnable example setup for the cafe domain, especially to understand how command dispatching and projectors are launched with `esrc-cqrs` and NATS.
    - Functions: main

- crates/esrc-cqrs/Cargo.toml
    - Summary: Cargo manifest for the esrc-cqrs crate, defining package metadata, feature flags, and dependencies for CQRS command/event handler registry support.
    - When To Use: Include this file when you need to understand the crate's build configuration, enabled features, or dependency relationships for the CQRS registry integration.

- examples/cafe/domain.rs
    - Summary: Defines a cafe order domain aggregate with order state, commands, events, error handling, and aggregate command/event application logic.
    - When To Use: Include this file when you need the core domain model for the cafe example, especially order lifecycle behavior, command processing, event definitions, or aggregate state transitions.
    - Types: OrderStatus, Order, OrderCommand, OrderEvent, OrderError
    - Functions: process, apply

- crates/esrc-cqrs/tests/integration_nats.rs
    - Summary: Integration tests for esrc-cqrs against a live NATS JetStream server, covering command dispatch, durable event storage, projector behavior, error propagation, malformed payload handling, and registry accessors.
    - When To Use: Include this file when you need to understand or verify the NATS/JetStream integration behavior of esrc-cqrs, especially request/reply command handling, projector execution, durability, or end-to-end test setup.
    - Types: Counter, CounterCommand, CounterEvent, CounterError, RecordingProjector, ProjectorError
    - Functions: unique_prefix, make_store, leak, test_command_request_response_success, test_command_error_does_not_break_dispatcher, test_projector_receives_events, test_projector_error_propagates, test_multiple_commands_same_aggregate_occ, test_malformed_payload_returns_error, test_registry_accessors

