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

- src/error.rs
    - Summary: Defines the crate’s common event-sourcing error type and a Result alias for operations that can fail with these errors.
    - When To Use: Include this file when handling, matching, or propagating the library’s standardized errors, or when using the event-sourcing Result alias.
    - Types: Error, Result

- src/event/subscribe.rs
    - Summary: Defines async subscription traits for event streams and a helper extension to observe subscribed events by projecting them with a Project implementation.
    - When To Use: Include this file when working with event subscription APIs, consuming new events from streams, or projecting incoming events into application state via SubscribeExt::observe.
    - Types: Subscribe, SubscribeExt
    - Functions: subscribe, observe

- src/nats/event.rs
    - Summary: Implements NATS-backed event store operations for publishing, replaying, subscribing, durable observation, and truncation of events, including header metadata handling and envelope conversion.
    - When To Use: Use when you need the NATS event store behavior, especially for publishing events with headers, replaying or subscribing to event streams, durable projection consumption, or truncating aggregate streams.
    - Types: NatsStore, NatsEnvelope
    - Functions: NatsStore::durable_observe, Replay::replay, ReplayOne::replay_one, Subscribe::subscribe, Truncate::truncate

- Cargo.toml
    - Summary: Workspace and root package manifest for the esrc project, which provides primitives for event sourcing and CQRS. It defines shared dependencies, workspace members (derive, opentelemetry-nats), and feature flags for NATS and KurrentDB integrations.
    - When To Use: Use this file to understand the project's dependency tree, available feature configurations (like 'nats' or 'kurrent'), and workspace structure for event-sourcing implementations.

- src/lib.rs
    - Summary: Crate root for the event-sourcing library, declaring core modules for aggregates, envelopes, errors, events, projections, and versioning, plus optional event store integrations.
    - When To Use: Use this file to understand the library’s overall module layout, available top-level re-exports, and which backend integrations are conditionally compiled.
    - Types: Aggregate, Envelope, Error, Event, EventGroup

- src/event.rs
    - Summary: Defines the core event abstractions for the crate, including the Event and EventGroup traits, the Sequence struct for stream ordering, and re-exports for publish, replay, subscribe, truncate, and command service operations.
    - When To Use: Use this file to define domain events, handle stream sequences, or access the primary traits for interacting with an event store.
    - Types: Sequence, Event, EventGroup, Publish, PublishExt, Replay, ReplayExt, ReplayOne, ReplayOneExt, Subscribe, SubscribeExt, Truncate, CommandError, CommandErrorKind, CommandService
    - Functions: Sequence::new

- src/event/command_service.rs
    - Summary: Defines the CommandService trait and associated error structures for processing commands against event-sourced aggregates, specifically designed for integration with NATS services.
    - When To Use: Include this file when implementing command processing logic, defining aggregate-based service endpoints, or handling error responses from command execution.
    - Types: CommandError, CommandErrorKind, CommandService

- src/nats.rs
    - Summary: Defines the NatsStore, a JetStream-backed event store implementation. It manages stream and mirror configuration, durable/ordered consumer setup, and provides a GracefulShutdown mechanism for managing asynchronous tasks.
    - When To Use: Include when using NATS JetStream as an event store backend, requiring management of streams, consumers, or coordinated task shutdown.
    - Types: NatsStore, GracefulShutdown, NatsEnvelope
    - Functions: NatsStore::try_new, NatsStore::enable_mirror, NatsStore::get_task_tracker, NatsStore::wait_graceful_shutdown, NatsStore::update_durable_consumer_option, NatsStore::client

- src/nats/command_service.rs
    - Summary: Implements the CommandService trait for NatsStore, providing a NATS-based microservice that listens for commands, replays aggregate state, and processes updates using optimistic concurrency.
    - When To Use: Use this file to understand or modify how the system handles incoming command requests via NATS, specifically the lifecycle of fetching state, executing domain logic, and replying to the caller.
    - Functions: serve

