# Event modeling workflow

The `event_modeling` module provides a declaration layer for vertical slices, while the runtime execution remains owned by infrastructure.

## Declaration layer

Vertical slices should declare consumers using the `src/event_modeling.rs` types:

- `ConsumerName`
- `ConsumerSpec<P>`
- `Automation<P>`
- `ReadModel<P>`

A slice declaration should express:

- the concrete projector type
- the semantic role
- the stable structured consumer identity
- any execution policy override

A slice should not deal with:

- durable consumer creation
- NATS subject derivation
- graceful shutdown wiring
- task spawning
- message stream lifecycle

This keeps feature code focused on intent rather than transport mechanics.

## Runtime layer

The runtime layer is currently implemented by `NatsStore` in `src/nats.rs` and `src/nats/event.rs`.

Infrastructure is responsible for:

- deriving the durable consumer name from `ConsumerName`
- deriving event subjects from `<P as Project>::EventGroup::names()`
- creating the durable consumer
- executing the consumer according to its `ExecutionPolicy`
- converting envelopes into typed `project::Context`
- mapping projector failures into crate `Error`
- acknowledging successfully processed messages
- spawning background tasks and logging runtime failures

This preserves the intended split from the design discussion, slices declare consumers, infrastructure runs them.

## Why `Automation<P>` and `ReadModel<P>` remain explicit

The current API intentionally keeps explicit slice-facing wrappers:

- `Automation<P>`
- `ReadModel<P>`

These wrappers still normalize into `ConsumerSpec<P>` for runtime use, but they remain valuable because they make the consumer role obvious at the declaration site.

This preserves the ergonomic direction discussed in the dev chat:

- automations are workflow-oriented consumers and default to concurrent execution
- read models are state-building consumers and default to sequential execution

The runtime does not duplicate execution machinery by role. Instead, both wrappers feed the same generic consumer execution path through `ConsumerSpec<P>` and `ExecutionPolicy`.

## Structured naming model

`ConsumerName` captures four stable naming segments:

- bounded context
- domain
- feature
- consumer

The durable consumer name is derived as:

- `bounded_context.domain.feature.consumer`

The slice path is derived as:

- `bounded_context.domain.feature`

This replaces ad hoc naming strings with a stable operational identity that is consistent across startup, observability, and infrastructure configuration.

## Projector execution model after removing `DynProject`

The consumer runtime now executes declared consumers through generic projector execution instead of a dynamic `DynProject` trait object.

The retained execution model is:

- `ConsumerSpec<P>`, `Automation<P>`, and `ReadModel<P>` stay generic over the concrete projector type `P`
- `NatsStore::run_consumer` is generic over `P`
- startup subject discovery uses `<P as Project>::EventGroup`
- typed `Context` construction happens inside the runtime from each incoming `NatsEnvelope`
- sequential execution owns one mutable projector instance for the lifetime of the consumer loop
- concurrent execution clones the projector for each in-flight task

This replaced the previous `DynProject` direction because the runtime boundary did not actually need fully erased per-message projector dispatch.

The earlier dynamic approach mixed several concerns that do not fit well into one erased trait surface:

- object safety
- associated event group typing
- typed `Context` construction
- async projector dispatch

The generic execution path fits the actual runtime domain better because the runtime only needs to do three things:

- discover the event names for startup
- own projector values with clone semantics appropriate to the execution policy
- call the existing typed `Project` implementation for each envelope

This keeps the implementation compile-safe and avoids forcing typed event processing through an erased abstraction that does not match the message handling boundary.

## Retained design decisions from the dev chat

The current implementation keeps the main design choices that were selected during the dev chat.

### Kept as infrastructure details

The runtime still keeps these concerns private to infrastructure:

- durable subscription creation
- consumer startup wiring
- shared message processing
- shutdown ownership
- background task logging

This means vertical slices do not use low-level NATS subscription helpers directly.

### Shared runtime entrypoint

The runtime now has a shared entrypoint for declared consumers:

- `NatsStore::run_consumer`

And matching ergonomic startup helpers:

- `NatsStore::spawn_consumer`
- `NatsStore::spawn_automation`
- `NatsStore::spawn_read_model`

This follows the earlier recommendation to centralize runtime behavior instead of exposing separate low-level start functions as the main declaration model.

### Shared processing pipeline

The runtime still uses one shared execution pipeline for:

- envelope conversion
- typed context creation
- projector execution
- error mapping
- ack handling

The major change is only how the projector is represented during execution, generic `P` instead of erased `DynProject`.

### Wrapper builders preserved

The implementation kept the explicit wrapper-builder approach:

- `Automation::new(...)`
- `ReadModel::new(...)`

This intentionally preserves the ergonomic slice-facing API chosen earlier.

It also keeps the prior documented choice to skip the intermediate `ConsumerSpec::automation(...)` and `ConsumerSpec::read_model(...)` constructor step for now.

## Integration status of the updated projector model

The current generic projector execution model still supports the previously introduced declaration and runtime features:

- `ConsumerSpec<P>`
- `Automation<P>`
- `ReadModel<P>`
- sequential consumer execution
- concurrent consumer execution with bounded in-flight work

This means the projector abstraction changed, but the declaration layer versus runtime layer architecture remained intact.

## Typical usage shape

A vertical slice should still declare consumers in a way that reads like intent:

- define a `ConsumerName`
- wrap a concrete projector in `Automation<P>` or `ReadModel<P>`
- optionally override execution policy
- pass the declaration to `NatsStore`

The runtime remains responsible for turning that declaration into a durable NATS consumer and running the typed `Project` implementation safely.
