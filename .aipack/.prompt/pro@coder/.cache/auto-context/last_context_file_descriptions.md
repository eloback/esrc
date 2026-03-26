# Context Files Descriptions (Sent to AI for selection)

- src/event/future.rs
    - Summary: Defines the IntoSendFuture trait to address Rust compiler limitations (issue #100013) in verifying that a Future is Send.
    - When To Use: Include this file when working with generic asynchronous code or extension traits where the compiler fails to automatically confirm that a Future implements the Send trait.
    - Types: IntoSendFuture

- src/event/truncate.rs
    - Summary: Defines the Truncate trait used to delete or drop old messages from an event stream up to a specified sequence number.
    - When To Use: Include this file when implementing or interacting with event storage logic that requires stream pruning or size management, especially when combined with snapshotting.
    - Types: Truncate

- src/version.rs
    - Summary: Defines core traits and re-exports derive macros for versioned serialization and deserialization, allowing types to handle schema evolution by associating a version number with serialized data.
    - When To Use: Use this file when you need to define data structures (like events) that require versioning for backward compatibility and schema migration during deserialization.
    - Types: DeserializeVersion, SerializeVersion

- src/event/replay.rs
    - Summary: Defines traits and extension methods for replaying event streams to rebuild projections or materialize aggregates from historical events.
    - When To Use: Use this file when implementing event store replay logic, rebuilding read models (projections), or materializing the state of an aggregate from its event history.
    - Types: Replay, ReplayOne, ReplayExt, ReplayOneExt
    - Functions: replay, replay_one, rebuild, rebuild_after, read, read_after

- src/nats/subject.rs
    - Summary: Defines the NatsSubject enum for modeling NATS subjects (Wildcards, Events, or Aggregates) and provides utilities for string parsing and generation.
    - When To Use: Use this file when you need to parse incoming NATS subject strings into structured types or format subjects for publishing messages to NATS.
    - Types: NatsSubject
    - Functions: NatsSubject::try_from_str, NatsSubject::into_string

- src/envelope.rs
    - Summary: Defines the Envelope trait for accessing event metadata and data from a store, and the TryFromEnvelope trait for deserializing envelopes into specific event types.
    - When To Use: Include when working with event store backends, handling event streams, or implementing logic to convert generic event envelopes into concrete event types or groups.
    - Types: Envelope, TryFromEnvelope

- src/nats/header.rs
    - Summary: Provides constants for standard NATS header keys and a helper function to extract header values from NATS messages.
    - When To Use: Use when you need to retrieve metadata, version, or event type information from the headers of an async_nats::Message.
    - Functions: get

- src/nats/envelope.rs
    - Summary: Defines the NatsEnvelope struct and its implementation of the Envelope trait, providing a wrapper for NATS Jetstream messages to handle event metadata, versioned deserialization, and message acknowledgement.
    - When To Use: Use this file when consuming messages from NATS Jetstream that need to be converted into domain events, specifically for extracting sequence numbers, timestamps, and aggregate identifiers from NATS subjects and headers.
    - Types: NatsEnvelope
    - Functions: try_from_message, attach_span_context, ack

- src/event/subscribe.rs
    - Summary: Defines the core traits for subscribing to event streams and projecting newly published events in real-time.
    - When To Use: Include this file when you need to implement or consume an event subscription mechanism, or when you want to set up an observer that projects incoming events into a specific state.
    - Types: Subscribe, SubscribeExt
    - Functions: subscribe, observe

- src/event/publish.rs
    - Summary: Defines the core traits for publishing events to an event stream, including support for optimistic concurrency control (OCC) and integration with aggregate roots.
    - When To Use: Include this file when implementing event storage logic or when performing operations that require persisting events and updating aggregate state atomically.
    - Types: Publish, PublishExt
    - Functions: publish, publish_without_occ, write, try_write

- src/nats/convert.rs
    - Summary: Implements trait conversions (From) to map various NATS JetStream error types into the application's internal Error type.
    - When To Use: Refer to this file to understand how NATS errors (like PublishError or StreamError) are categorized and handled internally, specifically regarding the mapping to 'Conflict' or 'Internal' error variants.

- src/error.rs
    - Summary: Defines the central Error enum and Result type alias used throughout the event-sourcing library to handle internal, external, formatting, and concurrency issues.
    - When To Use: Include this file when defining or calling functions that may fail during event-sourcing operations, such as command processing, event projection, or stream interactions.
    - Types: Error, Result

- src/event.rs
    - Summary: Core module defining fundamental traits and types for event sourcing, such as Event and EventGroup, along with Sequence management. It acts as a central hub for event-related operations like publishing, replaying, and subscribing.
    - When To Use: Use this file when defining domain events, implementing event streams, managing optimistic concurrency via sequences, or exploring high-level event store interaction traits.
    - Types: Sequence, Event, EventGroup, EventGroupType, CommandClient, CommandService, Publish, PublishExt, Replay, ReplayExt, ReplayOne, ReplayOneExt, Subscribe, SubscribeExt, Truncate

- src/event/command_service.rs
    - Summary: Defines core traits for command handling in an event-sourced system, providing abstractions for serving commands (CommandService) and sending them (CommandClient).
    - When To Use: Use this file when implementing the infrastructure for command processing, such as a backend service that executes aggregate logic or a client that routes commands to those services.
    - Types: CommandService, CommandClient

- src/nats/event.rs
    - Summary: Implements core event sourcing operations for the NATS backend, including publishing events with optimistic concurrency control, replaying event streams, subscribing to event groups via durable consumers, and truncating aggregate history.
    - When To Use: Use this file when interacting with NATS JetStream as an event store, specifically for publishing events, setting up projectors/consumers, or replaying historical event data.
    - Functions: publish, publish_without_occ, durable_observe, replay, replay_one, subscribe, truncate

- src/project.rs
    - Summary: Defines the core abstractions for event projection, including the Project trait for handling events and the Context struct which wraps deserialized event data with envelope metadata.
    - When To Use: Include this file when you need to define how events are projected into read models or side effects, or when working with the context of a specific event being projected.
    - Types: Context, Project

- docs/skill/esrc-slice-constants-and-module-layout.md
    - Summary: Defines the architectural standards for vertical slice layouts and mandatory constants (BOUNDED_CONTEXT_NAME, DOMAIN_NAME, FEATURE_NAME) within an esrc-based project to ensure module isolation and consistent instrumentation.
    - When To Use: Refer to this when implementing new features, setting up bounded contexts, or organizing domain modules to ensure the project structure adheres to standardized naming and isolation rules.
    - Types: BOUNDED_CONTEXT_NAME, DOMAIN_NAME, FEATURE_NAME

- docs/skill/esrc-event-modeling-create-consumers-automations-and-read-models.md
    - Summary: This documentation outlines the process for declaring event-driven consumers (Automations and Read Models) using the esrc::event_modeling framework. It details naming conventions, execution policies, and the implementation of the Project trait for vertical slices.
    - When To Use: Refer to this file when implementing side-effect-heavy workflows (Automations) or state-materializing consumers (Read Models) within an event-sourced architecture.
    - Types: ConsumerName, ConsumerRole, ExecutionPolicy, ConsumerSpec, Automation, ReadModel, Project
    - Functions: ConsumerName::new, Automation::new, ReadModel::new, max_concurrency, with_execution_policy, into_spec

- src/nats/query_service.rs
    - Summary: Implements NATS-based query services and clients using request-reply patterns. It provides the logic to serve read models and custom queries over NATS, as well as the client implementation to invoke those queries.
    - When To Use: Use this file when dealing with NATS-backed query handlers, implementing read model retrieval, or managing the lifecycle of background query services within the NATS messaging layer.
    - Types: QueryRequest, QueryReplyError, GetByIdReply, QueryReply
    - Functions: serve, get_by_id, query, spawn_query_service

- src/aggregate.rs
    - Summary: Defines the core `Aggregate` trait and the `Root` struct, which are central to representing and managing domain objects that are constructed from event streams.
    - When To Use: Use this file when implementing domain logic that processes commands into events or when you need to manage an aggregate's state and metadata (like ID and sequence) in an event-sourced system.
    - Types: Aggregate, Root
    - Functions: process, apply, with_aggregate, id, last_sequence, into_inner, new, try_apply

- src/query/mod.rs
    - Summary: Defines core traits and types for declaring, handling, serving, and consuming queries against read models in an event-sourced architecture.
    - When To Use: Use this file when implementing read model query logic, defining query handlers, or configuring query transport and service/client infrastructure.
    - Types: Query, QueryHandler, QueryTransport, QuerySpec, QueryService, QueryClient
    - Functions: QuerySpec::new, QuerySpec::name, QuerySpec::transport, QuerySpec::handler, QuerySpec::handler_mut, QuerySpec::into_handler, QuerySpec::with_transport

- src/query/in_memory.rs
    - Summary: An in-memory, thread-safe store and query handler for read-model projections, allowing shared access between write-side projectors and read-side query handlers.
    - When To Use: Use when you need a simple, memory-backed storage for live projections or during testing to facilitate querying and updating read models without external database dependencies.
    - Types: InMemoryViewStore
    - Functions: new, upsert, remove, get, all, len, is_empty, get_by_id, handle

- src/nats/query_kv.rs
    - Summary: Implements a NATS JetStream Key-Value backed store for read models. It serves as a shared store that handles both the persistence (writing) of read model instances and the execution of queries (reading) as a QueryHandler.
    - When To Use: Use this file when you need to implement a read model store or a query handler using NATS JetStream KV buckets as the underlying storage mechanism.
    - Types: NatsKvStore, QueryFuture
    - Functions: NatsKvStore::new, NatsKvStore::with_bucket_name, NatsKvStore::from_context, NatsKvStore::put, NatsKvStore::delete, NatsKvStore::get, NatsKvStore::bucket

- src/event_modeling.rs
    - Summary: Defines core primitives and builders for event-driven consumers, including semantic roles, execution policies, structured component naming, and composite read-model slices.
    - When To Use: Use this file when defining or configuring event consumers (Automations, Read Models), defining vertical slices that combine event projection with query handling, or using the structured component naming convention for infrastructure resources.
    - Types: ConsumerRole, ExecutionPolicy, ComponentName, ConsumerSpec, Automation, ReadModel, ReadModelSlice
    - Functions: ConsumerRole::default_execution_policy, ComponentName::new, ComponentName::bounded_context, ComponentName::domain, ComponentName::feature, ComponentName::component, ComponentName::durable_name, ComponentName::query_subject, ComponentName::slice_path, ConsumerSpec::new, ConsumerSpec::name, ConsumerSpec::role, ConsumerSpec::execution_policy, ConsumerSpec::projector, ConsumerSpec::projector_mut, ConsumerSpec::into_projector, ConsumerSpec::with_execution_policy, Automation::new, Automation::with_execution_policy, Automation::max_concurrency, Automation::as_spec, Automation::into_spec, ReadModel::new, ReadModel::with_execution_policy, ReadModel::as_spec, ReadModel::into_spec, ReadModelSlice::new, ReadModelSlice::with_execution_policy, ReadModelSlice::with_query_transport, ReadModelSlice::consumer_spec, ReadModelSlice::query_spec, ReadModelSlice::into_specs, ReadModelSlice::name, ReadModelSlice::projector, ReadModelSlice::projector_mut, ReadModelSlice::handler, ReadModelSlice::handler_mut

- src/nats/command_service.rs
    - Summary: Implements NATS-based command handling for event-sourced aggregates, featuring a service that reconstructs state via replay to execute commands and a client for NATS request-reply communication.
    - When To Use: Use this file to set up a command processing listener for an aggregate or to send commands to a remote aggregate service over NATS.
    - Types: ReplyError, CommandReply
    - Functions: serve, spawn_service, handle_request, send_command

- Cargo.toml
    - Summary: Root manifest and workspace configuration for the esrc project, managing shared dependencies, sub-crates (derive and opentelemetry-nats), and feature flags for event-sourcing and CQRS functionality.
    - When To Use: Refer to this file to check available feature flags (nats, derive, opentelemetry), workspace structure, or the versions of external dependencies like tokio, async-nats, and serde.

- src/nats.rs
    - Summary: Implementation of a NATS JetStream-backed event store, providing stream management, consumer orchestration, and graceful shutdown handling for background tasks.
    - When To Use: Use this file to initialize the NATS event store or to spawn background tasks for event consumers, automations, and read model projections.
    - Types: NatsStore, GracefulShutdown, NatsEnvelope
    - Functions: try_new, enable_mirror, get_task_tracker, wait_graceful_shutdown, client, jetstream_context, run_consumer, spawn_consumer, spawn_automation, spawn_read_model, spawn_read_model_slice

- src/lib.rs
    - Summary: The crate root for the esrc library, defining the core module structure and re-exporting primary traits and types for event sourcing, including aggregates, envelopes, and event types.
    - When To Use: Include this file to understand the overall architecture of the library, identify available sub-modules (like NATS), or see the main public API entry points.
    - Types: Aggregate, Envelope, Error, Event, EventGroup

- docs/skill/esrc-command-service-execute-commands.md
    - Summary: A guide on using esrc's CommandClient and CommandService to execute commands against aggregates. It details the command execution model, error handling strategies (Conflict, External, Internal), and architectural patterns for slice-based command triggering.
    - When To Use: Use this file when a slice needs to trigger state changes via aggregates, coordinate workflows, or implement automation that reacts to events by sending commands.
    - Types: CommandClient, CommandService, Aggregate
    - Functions: send_command, serve, process, apply

- docs/skill/esrc-read-model-public-interface-and-queries.md
    - Summary: Documentation of the standard pattern for defining slice read models and queries using the esrc::query module, focusing on generated.rs for data structures and mod.rs for custom queries.
    - When To Use: Use this guide when implementing or modifying a slice's read model interface, defining public query structures, or establishing the boundary between internal storage and public API types.
    - Types: Query, QueryHandler, QuerySpec, QueryService, QueryClient

