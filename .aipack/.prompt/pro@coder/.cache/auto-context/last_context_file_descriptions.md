# Context Files Descriptions (Sent to AI for selection)

- examples/cafe/error.rs
    - Summary: Defines the Cafe example's tab-related error type used for tab state and payment validation.
    - When To Use: Include when working with the cafe example's error handling or any code that matches on or returns tab-related errors.
    - Types: TabError

- examples/cafe/tab.rs
    - Summary: Defines a cafe tab aggregate with commands, events, and state transitions for opening a tab, placing orders, marking items served, and closing the tab with payment validation.
    - When To Use: Use when you need the domain model and business rules for the cafe tab example, especially the command/event definitions or how the aggregate validates and applies tab lifecycle changes.
    - Types: Item, TabCommand, TabEvent, Tab

- examples/cafe/tab/tests.rs
    - Summary: Unit tests for the cafe tab aggregate, covering opening a tab, placing orders, marking items served, and closing tabs with both success and error cases.
    - When To Use: Include this file when you need to understand or validate the expected behavior of the Tab domain logic, event transitions, and error handling in the cafe example.
    - Functions: open_tab, order_with_unopened_tab, order, serve_twice, serve, close_with_underpayment, close

- examples/zero_copy/main.rs
    - Summary: Example showing how to define a zero-copy event type and a simple projector that observes events from a NATS JetStream-backed event store.
    - When To Use: Include this file when you need an example of event-store usage with serde zero-copy deserialization, the Project trait implementation, or wiring up a NATS store and observer in an async main.
    - Types: ZeroCopyEvent, NamePrinter, NamePrinterError
    - Functions: main

- examples/cafe/table.rs
    - Summary: Defines ActiveTables, a project state manager that tracks open cafe tables by UUID and table number, with helper queries for active tables.
    - When To Use: Use when you need to understand or modify how table open/close events are stored, queried, or projected into shared state in the cafe example.
    - Types: ActiveTables
    - Functions: new, is_active, get_table_numbers, project

- crates/esrc-cqrs/src/command.rs
    - Summary: Defines the CQRS command-handling trait used to route and process incoming command messages against an event store.
    - When To Use: Include this file when working with command dispatch, implementing new command handlers, or understanding how raw command payloads are handled and replied to in the CQRS layer.
    - Types: CommandHandler

- crates/esrc-cqrs/src/nats/projector_runner.rs
    - Summary: Defines a NATS-specific projector runner that wraps a ProjectorHandler and executes it against a NatsStore, typically in its own Tokio task for concurrent projector execution.
    - When To Use: Use when working with NATS-backed CQRS projector execution, especially to start or invoke a projector handler against a NatsStore.
    - Types: NatsProjectorRunner<H>
    - Functions: new(handler: H) -> Self, run(&self, store: &NatsStore) -> error::Result<()>

- crates/esrc-cqrs/src/projector.rs
    - Summary: Defines the ProjectorHandler trait used to run event projector subscriptions against an event store, including durable naming and execution logic.
    - When To Use: Include this file when working with CQRS projectors, subscription handlers, or code that needs to define/run durable or transient projector tasks over an event store.
    - Types: ProjectorHandler

- crates/esrc-cqrs/src/nats/aggregate_projector_handler.rs
    - Summary: Defines a durable NATS JetStream-backed projector handler that runs a `Project` via a named durable consumer, allowing resume from the last processed position after restarts.
    - When To Use: Include this file when working with NATS-based CQRS projection handlers, durable consumer setup, or the `ProjectorHandler` implementation for resuming projectors from persisted stream positions.
    - Types: DurableProjectorHandler<P>
    - Functions: DurableProjectorHandler::new(durable_name: &'static str, projector: P) -> Self

- examples/cafe/projector.rs
    - Summary: Defines `OrderProjector`, a `Project` implementation that handles `OrderEvent`s and prints order activity to stdout.
    - When To Use: Use this file when you need the projection logic for cafe order events, especially to understand how order events are rendered/logged during event handling.
    - Types: OrderProjector

- Cargo.toml
    - Summary: Workspace and root package manifest for the esrc Rust project, defining shared dependencies, feature flags, and member crates/examples.
    - When To Use: Use this file when you need to understand the project structure, enabled feature combinations, dependency versions, workspace members, or package metadata for builds and integration context.

- crates/esrc-cqrs/Cargo.toml
    - Summary: Cargo manifest for the esrc-cqrs crate, defining package metadata, feature flags, and dependencies for CQRS command/event handler registry support.
    - When To Use: Include this file when you need to understand the crate's build configuration, enabled features, or dependency relationships for the CQRS registry integration.

- crates/esrc-cqrs/src/nats/command_dispatcher.rs
    - Summary: Implements a NATS-based command dispatcher that registers erased command handlers as request/reply endpoints on a NATS service and forwards requests to them, plus a helper for building command subjects.
    - When To Use: Include this file when you need to understand or modify how CQRS command handlers are exposed over NATS, how requests are dispatched to handlers, or how command subject names are constructed.
    - Types: NatsCommandDispatcher
    - Functions: NatsCommandDispatcher::new, NatsCommandDispatcher::run, command_subject

- crates/esrc-cqrs/src/nats/aggregate_command_handler.rs
    - Summary: Defines a generic NATS-backed aggregate command handler plus request/reply envelopes for routing, deserializing, applying, and responding to aggregate commands.
    - When To Use: Use this file when you need to handle commands sent over NATS for an event-sourced aggregate, including loading the aggregate, applying a command, and returning a serialized success reply.
    - Types: CommandEnvelope<C>, CommandReply, AggregateCommandHandler<A>
    - Functions: AggregateCommandHandler::new, CommandHandler<NatsStore>::name, CommandHandler<NatsStore>::handle

- crates/esrc-cqrs/src/error.rs
    - Summary: Defines the serializable CQRS error type used for transport over NATS, plus conversion from the core esrc error type and helpers to recover typed external errors.
    - When To Use: Use when handling CQRS command errors, serializing/deserializing errors across the transport boundary, or converting between esrc::error::Error and the CQRS-specific serializable Error type.
    - Types: Error
    - Functions: from_esrc_error

- examples/cafe/domain.rs
    - Summary: Defines a cafe order domain aggregate with order state, commands, events, error handling, and aggregate command/event application logic.
    - When To Use: Include this file when you need the core domain model for the cafe example, especially order lifecycle behavior, command processing, event definitions, or aggregate state transitions.
    - Types: OrderStatus, Order, OrderCommand, OrderEvent, OrderError
    - Functions: process, apply

- examples/cafe/main.rs
    - Summary: Entry point for the cafe CQRS example using NATS/JetStream. It wires together the NATS store, command dispatcher, and durable projector, then sends sample order commands to demonstrate the flow.
    - When To Use: Include this file when you need the runnable example setup for the cafe domain, especially to understand how command dispatching and projectors are launched with `esrc-cqrs` and NATS.
    - Functions: main

- crates/esrc-cqrs/tests/integration_nats.rs
    - Summary: Integration tests for esrc-cqrs against a live NATS JetStream server, covering command dispatch, durable event storage, projector behavior, error propagation, malformed payload handling, query handling, and registry accessors.
    - When To Use: Include this file when you need to understand or verify the NATS/JetStream integration behavior of esrc-cqrs, especially request/reply command and query handling, projector execution, durability, or end-to-end test setup.
    - Types: CounterState, Counter, CounterCommand, CounterEvent, CounterError, RecordingProjector, ProjectorError
    - Functions: test_command_request_response_success, test_command_error_does_not_break_dispatcher, test_projector_receives_events, test_projector_acks_messages_no_redelivery, test_projector_error_propagates, test_multiple_commands_same_aggregate_occ, test_malformed_payload_returns_error, test_registry_accessors, test_query_returns_aggregate_state, test_query_default_state_for_new_aggregate, test_query_malformed_payload_returns_error, test_registry_query_handlers_accessor

- crates/esrc-cqrs/src/lib.rs
    - Summary: Top-level library module for the `esrc-cqrs` crate. It documents the CQRS extension, exposes modules for command handling, queries, projectors, the main registry, and optional NATS integrations, and re-exports the primary CQRS traits and registry type.
    - When To Use: Include this file when you need the crate’s public API surface, an overview of CQRS support, or to find where command handlers, query handlers, projector handlers, and the registry are defined and re-exported.
    - Types: CommandHandler, Error, ProjectorHandler, CqrsRegistry, QueryHandler

- crates/esrc-cqrs/src/nats/aggregate_query_handler.rs
    - Summary: Defines a generic NATS-backed CQRS query handler for aggregates, including standard query/reply envelopes and a projection-based handler that loads an aggregate root and serializes a response.
    - When To Use: Use when you need to route NATS queries to an aggregate, deserialize a query envelope containing an aggregate ID, read the aggregate from NATS storage, and return a JSON-serialized projected result.
    - Types: QueryEnvelope, QueryReply, ProjectFn, AggregateQueryHandler
    - Functions: AggregateQueryHandler::new, QueryHandler::name, QueryHandler::handle

- crates/esrc-cqrs/src/nats/query_dispatcher.rs
    - Summary: Implements a NATS-based query dispatcher that registers each query handler as a service endpoint and forwards request/reply queries to erased handlers. Also provides a helper to build query subjects.
    - When To Use: Use this file when working on CQRS query transport over NATS, especially for starting the query service, wiring handlers into endpoints, or constructing query subject names.
    - Types: NatsQueryDispatcher
    - Functions: query_subject

- crates/esrc-cqrs/src/query.rs
    - Summary: Defines the QueryHandler trait for handling read-only query messages by name, using a shared event store reference and returning serialized response bytes.
    - When To Use: Include this file when working on CQRS query handling, query routing by handler name, or implementing/consuming read-only handlers that deserialize request payloads and serialize replies.
    - Types: QueryHandler

- crates/esrc-cqrs/src/registry.rs
    - Summary: Defines a CQRS registry that stores command, projector, and query handlers, provides registration/accessors, and can spawn all projectors as Tokio background tasks. Also includes object-safe erased wrapper traits for heterogeneous command, projector, and query handlers.
    - When To Use: Include this file when you need to understand or use the CQRS handler registry, register command/projector/query implementations, access registered handlers or the shared store, or run projectors concurrently.
    - Types: CqrsRegistry<S>, ErasedCommandHandler<S>, ErasedProjectorHandler<S>, ErasedQueryHandler<S>
    - Functions: CqrsRegistry::new, CqrsRegistry::register_command, CqrsRegistry::register_projector, CqrsRegistry::register_query, CqrsRegistry::command_handlers, CqrsRegistry::projector_handlers, CqrsRegistry::query_handlers, CqrsRegistry::store, CqrsRegistry::run_projectors

- crates/esrc-cqrs/src/nats/mod.rs
    - Summary: NATS CQRS integration module that wires together command dispatching over core NATS request/reply, query dispatching, and projector execution over JetStream durable pull consumers.
    - When To Use: Include this file when you need the NATS-backed CQRS entry points, especially to understand or import the dispatcher and projector runner types re-exported from this module.
    - Types: AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler, AggregateQueryHandler, QueryEnvelope, QueryReply, NatsCommandDispatcher, NatsQueryDispatcher, NatsProjectorRunner

