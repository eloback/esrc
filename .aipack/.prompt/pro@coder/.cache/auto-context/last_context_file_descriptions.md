# Context Files Descriptions (Sent to AI for selection)

- src/event/future.rs
    - Summary: Defines the IntoSendFuture trait to address Rust compiler limitations (issue #100013) in verifying that a Future is Send.
    - When To Use: Include this file when working with generic asynchronous code or extension traits where the compiler fails to automatically confirm that a Future implements the Send trait.
    - Types: IntoSendFuture

- src/event/truncate.rs
    - Summary: Defines the Truncate trait used to delete or drop old messages from an event stream up to a specified sequence number.
    - When To Use: Include this file when implementing or interacting with event storage logic that requires stream pruning or size management, especially when combined with snapshotting.
    - Types: Truncate

- src/kurrent/subject.rs
    - Summary: Defines the KurrentSubject enum and methods for parsing and formatting event subjects in the Kurrent system, supporting wildcards, simple events, and aggregate-specific subjects.
    - When To Use: Include this file when you need to handle event subject identification, specifically for parsing subject strings into structured types or serializing them back into strings.
    - Types: KurrentSubject
    - Functions: try_from_str, into_string

- src/kurrent.rs
    - Summary: Defines the KurrentStore struct and associated modules, serving as the primary handle for an event store implementation built on top of KurrentDB.
    - When To Use: Included when configuring or interacting with KurrentDB as an event store backend, or when working with KurrentDB-specific event envelopes.
    - Types: KurrentStore
    - Functions: KurrentStore::try_new

- src/kurrent/header.rs
    - Summary: Defines constant strings for Kurrent-specific header keys used in event versioning and identification.
    - When To Use: Include this file when you need to reference the standard header keys 'Esrc-Version' or 'Esrc-Event-Type' within the Kurrent system.

- examples/zero_copy/main.rs
    - Summary: An example demonstrating how events can use zero-copy deserialization by tying event lifetimes to the source Envelope within the esrc framework.
    - When To Use: Use this file as a reference for implementing memory-efficient event projection or when working with HRTB (Higher-Rank Trait Bounds) for event lifetimes.
    - Types: ZeroCopyEvent, NamePrinter, NamePrinterError
    - Functions: main

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

- src/kurrent/event.rs
    - Summary: Implements event sourcing traits for KurrentStore, providing the logic for publishing events, replaying aggregate streams, and subscribing to event groups via KurrentDB (EventStoreDB). It also includes specialized methods for durable/persistent subscriptions and projections.
    - When To Use: Include this file when you need to interact with KurrentDB for publishing events, replaying history for a specific entity, or setting up persistent event consumers (projectors).
    - Functions: publish, publish_without_occ, replay_one, subscribe, durable_subscribe, durable_observe

- src/envelope.rs
    - Summary: Defines the Envelope trait for accessing event metadata and data from a store, and the TryFromEnvelope trait for deserializing envelopes into specific event types.
    - When To Use: Include when working with event store backends, handling event streams, or implementing logic to convert generic event envelopes into concrete event types or groups.
    - Types: Envelope, TryFromEnvelope

- src/kurrent/envelope.rs
    - Summary: Implements the Envelope trait for KurrentDB, providing a structure to wrap KurrentDB's ResolvedEvent and handle event metadata extraction and versioned deserialization.
    - When To Use: Use this file when processing events retrieved from KurrentDB to map them into the system's generic event envelope format.
    - Types: KurrentEnvelope
    - Functions: KurrentEnvelope::try_from_message

- src/nats/header.rs
    - Summary: Provides constants for standard NATS header keys and a helper function to extract header values from NATS messages.
    - When To Use: Use when you need to retrieve metadata, version, or event type information from the headers of an async_nats::Message.
    - Functions: get

- src/nats/envelope.rs
    - Summary: Defines the NatsEnvelope struct and its implementation of the Envelope trait, providing a wrapper for NATS Jetstream messages to handle event metadata, versioned deserialization, and message acknowledgement.
    - When To Use: Use this file when consuming messages from NATS Jetstream that need to be converted into domain events, specifically for extracting sequence numbers, timestamps, and aggregate identifiers from NATS subjects and headers.
    - Types: NatsEnvelope
    - Functions: try_from_message, attach_span_context, ack

- src/view.rs
    - Summary: Defines the View trait, which represents a reactive read model or projection built incrementally from an event stream for query purposes.
    - When To Use: Use this file when defining read-only projections of event data that react to events but do not require command processing or complex state management logic.
    - Types: View

- src/event/subscribe.rs
    - Summary: Defines the core traits for subscribing to event streams and projecting newly published events in real-time.
    - When To Use: Include this file when you need to implement or consume an event subscription mechanism, or when you want to set up an observer that projects incoming events into a specific state.
    - Types: Subscribe, SubscribeExt
    - Functions: subscribe, observe

- examples/cafe/domain.rs
    - Summary: Defines the core domain logic for a cafe order aggregate, including its state, commands, events, and error types, while implementing the esrc::Aggregate trait.
    - When To Use: Use this file when you need to reference the business logic, state transitions, or command/event schemas for the Order aggregate in the cafe example.
    - Types: OrderStatus, Order, OrderCommand, OrderEvent, OrderError

- src/event/publish.rs
    - Summary: Defines the core traits for publishing events to an event stream, including support for optimistic concurrency control (OCC) and integration with aggregate roots.
    - When To Use: Include this file when implementing event storage logic or when performing operations that require persisting events and updating aggregate state atomically.
    - Types: Publish, PublishExt
    - Functions: publish, publish_without_occ, write, try_write

- src/kurrent/convert.rs
    - Summary: Implements the From trait to convert kurrentdb::Error into the internal crate::error::Error type, mapping specific database errors like WrongExpectedVersion to application-level conflicts.
    - When To Use: Use this file when you need to understand or modify how external database errors from the Kurrent driver are translated into internal error variants.

- src/nats/convert.rs
    - Summary: Implements trait conversions (From) to map various NATS JetStream error types into the application's internal Error type.
    - When To Use: Refer to this file to understand how NATS errors (like PublishError or StreamError) are categorized and handled internally, specifically regarding the mapping to 'Conflict' or 'Internal' error variants.

- examples/cafe/main.rs
    - Summary: A practical example demonstrating the use of the esrc library with NATS JetStream. It shows how to initialize a NatsStore, spawn an aggregate command service, send commands through NATS, and retrieve aggregate state.
    - When To Use: Use this file as a reference for setting up a command service and interacting with an event-sourced aggregate using NATS as the transport and storage layer.
    - Functions: main

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

- examples/multi-slice-command-service/domain.rs
    - Summary: Defines the domain model for a multi-slice command service example, including aggregates, commands, events, and errors for Signup and Email contexts.
    - When To Use: Include this file when you need to reference the business logic, state transitions, or data structures for the Signup and Email aggregates in the multi-slice example.
    - Types: SignupEvent, EmailEvent, SignupCommand, EmailCommand, SignupError, EmailError, SignupAggregate, EmailAggregate

- src/nats/command_service.rs
    - Summary: Implements NATS-based command processing for aggregates, providing both a service (to listen and execute commands) and a client (to send commands via request/reply).
    - When To Use: Use this file when setting up a NATS microservice to handle domain commands for an aggregate or when you need to send commands to such a service from another part of the system.
    - Types: ReplyError, CommandReply
    - Functions: serve, spawn_service, handle_request, send_command

- examples/multi-slice-command-service/main.rs
    - Summary: Main entry point for a multi-slice command service example demonstrating aggregate service initialization, background automation (slices) setup, and command dispatching using NATS JetStream.
    - When To Use: Use this file as a template for bootstrapping an esrc application, specifically for initializing the NatsStore, spawning services, and coordinating multiple domain modules.
    - Types: BOUNDED_CONTEXT, SIGNUP_DOMAIN, EMAIL_DOMAIN
    - Functions: main

- examples/multi-slice-command-service/queue_welcome_email/mod.rs
    - Summary: Implements an automation component that reacts to SignupRequested events by dispatching commands to both the Email and Signup aggregates to coordinate a welcome email workflow.
    - When To Use: Use this file as a reference for implementing event-driven automations (sagas/process managers) that trigger side effects or commands across multiple aggregates in an esrc-based application.
    - Types: QueueWelcomeEmailAutomation
    - Functions: setup

- examples/multi-slice-command-service/send_welcome_email/mod.rs
    - Summary: Implements the SendWelcomeEmailAutomation, which acts as a process manager that listens for WelcomeEmailRequested events and responds by sending a MarkWelcomeEmailSent command.
    - When To Use: Include this file when looking for an example of how to implement event-driven automations or side effects within the esrc framework, particularly those that involve command-loopback patterns.
    - Types: SendWelcomeEmailAutomation
    - Functions: setup

- src/nats/query_service.rs
    - Summary: Implements NATS-based query services and clients using request-reply patterns. It provides the logic to serve read models and custom queries over NATS, as well as the client implementation to invoke those queries.
    - When To Use: Use this file when dealing with NATS-backed query handlers, implementing read model retrieval, or managing the lifecycle of background query services within the NATS messaging layer.
    - Types: QueryRequest, QueryReplyError, GetByIdReply, QueryReply
    - Functions: serve, get_by_id, query, spawn_query_service

- Cargo.toml
    - Summary: Workspace manifest and package configuration for esrc, a library providing primitives for event sourcing and CQRS systems.
    - When To Use: When needing to check dependency versions, feature flag definitions, or the structure of the workspace including its sub-crates (derive, opentelemetry-nats).

- examples/basic-query-service/domain.rs
    - Summary: Defines the core domain model for an Order system, including its state (Aggregate), the actions that can be taken (Commands), the resulting state changes (Events), and potential business logic errors.
    - When To Use: Use this file to understand or modify the business logic, state transitions, and event/command definitions for the order processing domain.
    - Types: OrderEvent, OrderCommand, OrderError, OrderAggregate

- examples/basic-query-service/main.rs
    - Summary: The main entry point for an example service demonstrating the full CQRS/ES lifecycle using NATS. It sets up an event store, spawns a command service for an order aggregate, establishes a read model projector, and runs a query service.
    - When To Use: Refer to this file when you need an end-to-end example of how to configure and run command services, read models, and query handlers using the esrc library.
    - Types: BOUNDED_CONTEXT, ORDER_DOMAIN
    - Functions: main

- examples/basic-query-service/read_model.rs
    - Summary: Defines the read-side components for an order service, including the OrderReadModel, an in-memory store, a projector to update state from events, and a query handler.
    - When To Use: Refer to this file when implementing CQRS read models, projections, or query handlers using the esrc framework.
    - Types: OrderReadModel, OrderQuery, OrderStore, OrderProjector, OrderQueryHandler
    - Functions: OrderStore::new, OrderStore::get, OrderStore::upsert, OrderStore::all, OrderProjector::new, OrderQueryHandler::new

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

- src/lib.rs
    - Summary: The crate root for the esrc library, defining the core module structure and re-exporting primary traits and types for event sourcing, including aggregates, envelopes, and views.
    - When To Use: Include this file to understand the overall architecture of the library, identify available sub-modules (like NATS or Kurrent integrations), or see the main public API entry points.
    - Types: Aggregate, Envelope, Error, Event, EventGroup, View

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

- src/nats.rs
    - Summary: Provides a NATS JetStream-backed event store implementation, handling stream configuration, consumer management, and graceful task lifecycle tracking.
    - When To Use: Use this file to initialize a NATS event store or to spawn background tasks for consumers, automations, and read model projections.
    - Types: NatsStore, GracefulShutdown, NatsEnvelope
    - Functions: try_new, enable_mirror, get_task_tracker, wait_graceful_shutdown, client, jetstream_context, run_consumer, spawn_consumer, spawn_automation, spawn_read_model, spawn_read_model_slice

