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

- src/aggregate.rs
    - Summary: Defines the core Aggregate trait for event-sourced domain objects and the Root wrapper that materializes an aggregate with stream identity and sequence tracking.
    - When To Use: Include when you need the aggregate abstraction, root aggregate state management, event application/validation, or to understand how commands are processed into events in this event-sourcing library.
    - Types: Aggregate, Root
    - Functions: Root::with_aggregate, Root::id, Root::last_sequence, Root::into_inner, Root::new, Root::try_apply

- src/envelope.rs
    - Summary: Defines the core Envelope abstraction for event-store records and a helper trait for converting envelopes into event types.
    - When To Use: Include this file when you need the event envelope interface, envelope metadata access, or automatic/manual deserialization from stored event data into domain events.
    - Types: Envelope, TryFromEnvelope

- src/event/publish.rs
    - Summary: Defines the event publishing traits for writing events to an event stream, including extension helpers that publish aggregate events and commands with optimistic concurrency support.
    - When To Use: Use this file when you need the API for publishing events, writing aggregate events back to the event store, or converting commands into events via aggregate helper methods.
    - Types: Publish, PublishExt
    - Functions: write, try_write

- src/event/replay.rs
    - Summary: Defines replay and replay extension traits for event streams, including helpers to rebuild projections and materialize aggregates from replayed events.
    - When To Use: Use this file when you need to replay events from an event store, rebuild a projector, or load/update an aggregate root from historical events.
    - Types: Replay, ReplayOne, ReplayExt, ReplayOneExt

- src/event/subscribe.rs
    - Summary: Defines async subscription traits for event streams and a helper extension to observe subscribed events by projecting them with a Project implementation.
    - When To Use: Include this file when working with event subscription APIs, consuming new events from streams, or projecting incoming events into application state via SubscribeExt::observe.
    - Types: Subscribe, SubscribeExt
    - Functions: subscribe, observe

- src/kurrent.rs
    - Summary: Defines the KurrentStore wrapper around a kurrentdb Client for use as an event store implementation, and exposes related modules for event conversion and envelopes.
    - When To Use: Include this file when you need the KurrentStore type, its construction, or to understand the KurrentDB-backed event store module structure.
    - Types: KurrentStore
    - Functions: KurrentStore::try_new

- src/kurrent/convert.rs
    - Summary: Implements conversion from kurrentdb errors into the crate's Error type, mapping optimistic concurrency conflicts to Conflict and all other errors to Internal.
    - When To Use: Include this file when working on error translation between kurrentdb and this crate, especially for handling expected-version conflicts.
    - Types: Error

- src/kurrent/envelope.rs
    - Summary: Defines KurrentEnvelope, an adapter that converts a kurrentdb ResolvedEvent into the crate’s Envelope trait implementation by extracting aggregate identity, sequence, timestamp, and version metadata, and providing version-aware event deserialization.
    - When To Use: Include this file when working with KurrentDB event ingestion, envelope conversion, or deserializing events from resolved KurrentDB records using the stream subject and version metadata.
    - Types: KurrentEnvelope
    - Functions: KurrentEnvelope::try_from_message

- src/kurrent/event.rs
    - Summary: Implements KurrentStore event publishing, replaying a single aggregate stream, and subscribing to event groups via KurrentDB persistent subscriptions, including durable subscribe/observe helpers.
    - When To Use: Use this file when working with KurrentDB-backed event store operations such as publishing events, reading aggregate history, creating persistent subscriptions, or running durable projector loops.
    - Functions: publish, publish_without_occ, replay_one, subscribe, durable_subscribe, durable_observe

- src/kurrent/header.rs
    - Summary: Defines constant header keys used for Kurrent/Esrc metadata, including version and event type headers.
    - When To Use: Include this file when working with code that reads, writes, or matches the custom Esrc/Kurrent HTTP or event metadata headers.

- src/kurrent/subject.rs
    - Summary: Defines the KurrentSubject enum for representing event subjects (wildcard, event name, or aggregate name with UUID) and provides parsing/string conversion helpers.
    - When To Use: Include this file when working with subject parsing, subject string formatting, or logic that distinguishes wildcard, event, and aggregate subjects in the Kurrent domain.
    - Types: KurrentSubject
    - Functions: try_from_str, into_string

- src/nats/convert.rs
    - Summary: Implements conversion from async_nats JetStream error types into the crate's unified Error type, including a special mapping for wrong last sequence publish errors to Conflict.
    - When To Use: Include this file when working on NATS/JetStream error handling, especially error conversion, mapping external NATS errors into the crate's Error type, or diagnosing publish/stream/consumer-related failures.

- src/nats/envelope.rs
    - Summary: Defines `NatsEnvelope`, an envelope wrapper for NATS JetStream messages that extracts aggregate metadata from the subject and headers, supports event deserialization by version, propagates tracing context, and acks messages on drop/use.
    - When To Use: Include this file when working with NATS JetStream event ingestion, message-to-envelope conversion, extracting event metadata from NATS headers/subjects, or deserializing versioned events from NATS payloads.
    - Types: NatsEnvelope
    - Functions: NatsEnvelope::try_from_message, NatsEnvelope::attach_span_context, NatsEnvelope::ack

- src/nats/header.rs
    - Summary: Defines NATS header key constants and a helper for reading header values from an async_nats message.
    - When To Use: Use this file when working with NATS message headers, especially to access standardized event/version metadata or to extract header strings from messages.
    - Types: VERSION_KEY, EVENT_TYPE, METADATA_PREFIX
    - Functions: get

- src/nats/subject.rs
    - Summary: Defines NATS subject variants and helpers for parsing and formatting subjects with a required prefix, event name, and optional aggregate UUID.
    - When To Use: Use when working with NATS subject routing in this crate, especially to parse incoming subject strings or build outgoing subject strings from domain values.
    - Types: NatsSubject
    - Functions: try_from_str, into_string

- src/version.rs
    - Summary: Defines version-aware serde extension traits for serializing and deserializing types with an associated version, and re-exports derive helpers when the derive feature is enabled.
    - When To Use: Include this file when working with versioned serialization/deserialization logic, serde integration, or when needing the `SerializeVersion`/`DeserializeVersion` traits (or their derive macros under the derive feature).
    - Types: DeserializeVersion, SerializeVersion

- src/view.rs
    - Summary: Defines the `View` trait, a lightweight reactive read model built from an event stream for query/projection purposes.
    - When To Use: Include this file when working with event-sourced read models, projections, or any code that implements or uses the `View` trait to replay/apply events.
    - Types: View

- Cargo.toml
    - Summary: Workspace and root package manifest for the esrc project, defining shared dependencies, workspace members (derive, opentelemetry-nats), and feature flags for NATS and KurrentDB integrations.
    - When To Use: Use this file to understand the project's dependency tree, available feature configurations, and workspace structure for event-sourcing implementations.

- src/event/command_service.rs
    - Summary: Defines traits for serving and interacting with command-handling services for event-sourced aggregates.
    - When To Use: Use this file when implementing a service to process aggregate commands or a client to send commands to such a service.
    - Types: CommandService, CommandClient

- src/nats/command_service.rs
    - Summary: Implements a NATS-based command service for event-sourced aggregates, providing a listener that replays aggregate state to process commands and a client for sending commands via the NATS request/reply pattern.
    - When To Use: Use this file when configuring NATS command handlers for aggregates, spawning background command services, or using the NatsStore to dispatch commands.
    - Types: ReplyError, CommandReply
    - Functions: serve, spawn_service, handle_request, send_command

- src/lib.rs
    - Summary: The crate root for the event-sourcing library, defining core modules and re-exporting primary types like Aggregate, Envelope, Event, and View.
    - When To Use: Use this file to understand the library's module structure, re-exported types, and feature-gated backend implementations for NATS and Kurrentdb.
    - Types: Aggregate, Envelope, Error, Event, EventGroup, View

- src/error.rs
    - Summary: Defines the central Error enum and Result alias for event-sourcing, including transport, formatting, and optimistic concurrency variants.
    - When To Use: Include when handling or returning errors from event-sourcing operations, especially command processing and event projection.
    - Types: Error, Result

- src/event.rs
    - Summary: Defines the core abstractions for an event-sourced system, including the Event, EventGroup, and EventGroupType traits, the Sequence struct for stream ordering, and re-exports for command handling, publishing, replaying, subscribing, and truncating events.
    - When To Use: Use this file to define domain events, handle event stream sequencing, or access the primary traits for interacting with an event store.
    - Types: CommandClient, CommandService, Event, EventGroup, EventGroupType, Publish, PublishExt, Replay, ReplayExt, ReplayOne, ReplayOneExt, Sequence, Subscribe, SubscribeExt, Truncate
    - Functions: Sequence::new

- src/event_modeling.rs
    - Summary: Defines core types and builders for modeling event consumers, including roles (Automation and ReadModel), execution policies (Sequential and Concurrent), and structured naming conventions for consumers.
    - When To Use: Use this file when defining new event consumers, configuring how messages should be processed (concurrency vs sequential), or establishing identity structures for consumers within bounded contexts.
    - Types: Automation, ConsumerName, ConsumerRole, ConsumerSpec, ExecutionPolicy, ReadModel

- src/nats/event.rs
    - Summary: Implements core NATS JetStream operations for NatsStore, including event publishing with OCC, stream replay, subscriptions, and durable consumer management for projections.
    - When To Use: Include this file when NATS-based event persistence, event publishing logic, or projection synchronization via JetStream is required.
    - Functions: Publish::publish, Publish::publish_without_occ, NatsStore::durable_observe, Replay::replay, ReplayOne::replay_one, Subscribe::subscribe, Truncate::truncate

- src/project.rs
    - Summary: Defines the projection API for event-sourced processing, including the Project trait for handling events, the DynProject trait for dynamic dispatch, and the Context wrapper that provides access to both deserialized event data and envelope metadata.
    - When To Use: Include this file when implementing event projectors to build read models, handling side effects from events, or when needing to access envelope-level metadata like timestamps and IDs within a projection.
    - Types: Context, Project, DynProject
    - Functions: Context::try_with_envelope, Context::id, Context::sequence, Context::timestamp, Context::get_metadata, Context::into_inner

- src/nats.rs
    - Summary: Implementation of a NATS JetStream-backed event store, including stream configuration, durable/ordered consumer management, and graceful shutdown tracking for event-sourced tasks.
    - When To Use: Use when interacting with NATS as an event store backend, specifically for managing streams, mirrors, and spawning background consumers for automations or read models.
    - Types: NatsStore, GracefulShutdown, NatsEnvelope
    - Functions: NatsStore::try_new, NatsStore::enable_mirror, NatsStore::get_task_tracker, NatsStore::wait_graceful_shutdown, NatsStore::update_durable_consumer_option, NatsStore::client, NatsStore::run_consumer, NatsStore::spawn_consumer, NatsStore::spawn_automation, NatsStore::spawn_read_model

