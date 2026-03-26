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

- derive/src/event.rs
    - Summary: Implements procedural macro helpers for deriving `Event` and `EventGroup` behavior from Rust types, including event name generation, enum variant type reporting, and skipping ignored event-group variants.
    - When To Use: Use when you need to understand or modify the derive macros that generate `::esrc::event::Event` and `::esrc::event::EventGroup` impls for structs/enums, especially around event naming rules, suffix stripping, and variant collection.
    - Types: EventMeta
    - Functions: derive_event, derive_event_group

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

- src/error.rs
    - Summary: Defines the central Error enum and Result alias for event-sourcing, including transport, formatting, and optimistic concurrency variants.
    - When To Use: Include when handling or returning errors from event-sourcing operations, especially command processing and event projection.
    - Types: Error, Result

- src/event.rs
    - Summary: Defines the core abstractions for an event-sourced system, including the Event, EventGroup, and EventGroupType traits, the Sequence struct for stream ordering, and re-exports for command handling, publishing, replaying, subscribing, and truncating events.
    - When To Use: Use this file to define domain events, handle event stream sequencing, or access the primary traits for interacting with an event store.
    - Types: CommandClient, CommandService, Event, EventGroup, EventGroupType, Publish, PublishExt, Replay, ReplayExt, ReplayOne, ReplayOneExt, Sequence, Subscribe, SubscribeExt, Truncate
    - Functions: Sequence::new

- src/event/command_service.rs
    - Summary: Defines traits for serving and interacting with command-handling services for event-sourced aggregates.
    - When To Use: Use this file when implementing a service to process aggregate commands or a client to send commands to such a service.
    - Types: CommandService, CommandClient

- src/nats/event.rs
    - Summary: Implements NATS JetStream-backed event operations for NatsStore, including event publishing with Optimistic Concurrency Control (OCC), stream replaying, durable subscriptions, and stream truncation.
    - When To Use: Include this file when NATS-based event persistence, stream replaying, or durable projection synchronization via JetStream is required in the system.
    - Functions: publish, publish_without_occ, durable_observe, replay, replay_one, subscribe, truncate

- src/project.rs
    - Summary: Defines the Project trait for event projection and a Context wrapper that encapsulates both the deserialized event and its associated envelope metadata.
    - When To Use: Use this file when implementing event projectors to build read models or handle side effects, particularly when you need access to envelope metadata like timestamps and sequence numbers during processing.
    - Types: Context, Project
    - Functions: Context::try_with_envelope, Context::id, Context::sequence, Context::timestamp, Context::get_metadata, Context::into_inner, Project::project

- Cargo.toml
    - Summary: Root workspace manifest for the esrc project, defining shared dependencies, workspace members (derive, opentelemetry-nats), and feature flags for NATS and KurrentDB integrations.
    - When To Use: Use this file to identify available feature flags, workspace structure, and specific dependency versions for event-sourcing and CQRS implementations.

- examples/multi-slice-command-service/domain.rs
    - Summary: Defines the domain logic for a multi-slice command service, including events, commands, errors, and aggregate state transitions for 'Signup' and 'Email' domains.
    - When To Use: Use this file as a reference for defining domain entities and business logic in an Event Sourcing system with multiple aggregates.
    - Types: SignupEvent, EmailEvent, SignupCommand, EmailCommand, SignupError, EmailError, SignupAggregate, EmailAggregate

- src/nats.rs
    - Summary: Implementation of a NATS JetStream-backed event store, providing functionality for stream management, durable/ordered consumer creation, and lifecycle management for event-processing tasks.
    - When To Use: Use when integrating NATS as the event storage backend, specifically for initializing streams, managing read mirrors, and spawning background consumers for read models or automations.
    - Types: NatsStore, GracefulShutdown, NatsEnvelope
    - Functions: NatsStore::try_new, NatsStore::enable_mirror, NatsStore::get_task_tracker, NatsStore::wait_graceful_shutdown, NatsStore::client, NatsStore::run_consumer, NatsStore::spawn_consumer, NatsStore::spawn_automation, NatsStore::spawn_read_model

- src/nats/command_service.rs
    - Summary: Implements a NATS-based command service for event-sourced aggregates, providing a listener that replays aggregate state to process commands and a client for sending commands via the NATS request/reply pattern.
    - When To Use: Use this file when configuring NATS command handlers for aggregates, spawning background command services, or using the NatsStore to dispatch commands.
    - Types: ReplyError, CommandReply
    - Functions: serve, spawn_service, handle_request, send_command

- examples/multi-slice-command-service/main.rs
    - Summary: The main entry point for a multi-slice command service example. It orchestrates the initialization of a NATS-backed event store, registers multiple aggregates (Signup and Email), sets up event-driven automation slices, and demonstrates a basic command-processing workflow.
    - When To Use: Refer to this file when setting up the bootstrapping logic for an application using esrc, specifically for configuring NatsStore, spawning aggregate services, and connecting automation logic.

- examples/multi-slice-command-service/queue_welcome_email/mod.rs
    - Summary: Implements the QueueWelcomeEmailAutomation which reacts to 'SignupRequested' events by triggering commands across the Email and Signup aggregates.
    - When To Use: Use this file as an example of how to implement cross-aggregate automations (sagas) in an event-sourced system using the esrc library.
    - Types: QueueWelcomeEmailAutomation
    - Functions: setup

- examples/multi-slice-command-service/send_welcome_email/mod.rs
    - Summary: Implements the SendWelcomeEmailAutomation, which handles the side-effect of 'sending' an email by reacting to a WelcomeEmailRequested event and issuing a MarkWelcomeEmailSent command.
    - When To Use: Include this file when examining examples of event-driven automations or process managers that coordinate actions between events and commands within the esrc framework.
    - Types: SendWelcomeEmailAutomation
    - Functions: setup

- src/lib.rs
    - Summary: Crate root for the esrc event-sourcing library, defining the core module structure and re-exporting primary types such as Aggregate, Envelope, Error, Event, EventGroup, and View.
    - When To Use: Use this file to understand the library's organization, available modules, and feature-gated backend implementations for NATS and Kurrentdb.
    - Types: Aggregate, Envelope, Error, Event, EventGroup, View

- src/event_modeling.rs
    - Summary: Defines core types and builders for modeling event consumers, including roles (Automation, ReadModel), execution policies (Sequential, Concurrent), and structured component naming (ComponentName).
    - When To Use: Use when defining event consumers and their execution policies, or when establishing structured component identities across bounded contexts.
    - Types: Automation, ComponentName, ConsumerRole, ConsumerSpec, ExecutionPolicy, ReadModel

- src/query.rs
    - Summary: Defines traits and types for declaring, handling, serving, and invoking queries against read models, providing infrastructure for remote query execution via transports like NATS.
    - When To Use: Use this file when defining domain-specific queries, implementing read model handlers, or configuring query services and clients for distributed communication.
    - Types: Query, QueryHandler, QueryTransport, QuerySpec, QueryService, QueryClient
    - Functions: QuerySpec::new, QuerySpec::name, QuerySpec::transport, QuerySpec::handler, QuerySpec::handler_mut, QuerySpec::into_handler, QuerySpec::with_transport

