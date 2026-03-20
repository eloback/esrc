# Context Files Descriptions (Sent to AI for selection)

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

- crates/esrc-cqrs/src/nats/aggregate_projector_handler.rs
    - Summary: Defines a durable NATS JetStream-backed projector handler that runs a `Project` via a named durable consumer, allowing resume from the last processed position after restarts.
    - When To Use: Include this file when working with NATS-based CQRS projection handlers, durable consumer setup, or the `ProjectorHandler` implementation for resuming projectors from persisted stream positions.
    - Types: DurableProjectorHandler<P>
    - Functions: DurableProjectorHandler::new(durable_name: &'static str, projector: P) -> Self

- crates/esrc-cqrs/Cargo.toml
    - Summary: Cargo manifest for the esrc-cqrs crate, defining package metadata, feature flags, and dependencies for CQRS command/event handler registry support.
    - When To Use: Include this file when you need to understand the crate's build configuration, enabled features, or dependency relationships for the CQRS registry integration.

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

- Cargo.toml
    - Summary: Workspace and root package manifest for the esrc Rust project, defining shared dependencies, feature flags, and member crates/examples.
    - When To Use: Use this file when you need to understand the project structure, enabled feature combinations, dependency versions, workspace members, or package metadata for builds and integration context.

