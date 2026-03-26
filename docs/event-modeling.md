# Event modeling workflow

This guide documents how the consumer declaration workflow is split between declaration owned by vertical slices and runtime execution owned by infrastructure.

## Declaration layer

Vertical slices should declare consumers through the `event_modeling` module.

The current declaration surface is:

- `ConsumerName`
- `ConsumerSpec<P>`
- `Automation<P>`
- `ReadModel<P>`

A slice declaration should describe:

- which projector is used
- which semantic role the consumer has
- which stable structured name identifies it
- which execution override, if any, should be applied

A slice declaration should not be responsible for:

- durable consumer creation
- message stream setup
- graceful shutdown wiring
- task spawning
- transport-specific runtime details

### Structured naming

`ConsumerName` captures these naming segments:

- bounded context
- domain
- feature
- consumer

That naming model produces a stable durable consumer identity in this form:

- `bounded_context.domain.feature.consumer`

This replaces ad hoc string naming with a structured operational identity that can stay stable across deployments.

## Runtime layer

The runtime layer is owned by infrastructure and is currently executed through `NatsStore`.

The runtime is responsible for:

- deriving the durable consumer name from `ConsumerName`
- deriving event subjects from `P::EventGroup`
- creating the durable NATS consumer
- selecting the execution path from the declared execution policy
- owning background task lifecycle and error reporting

This preserves the original design goal that vertical slices should declare intent while infrastructure owns transport and lifecycle behavior.

## Why `Automation<P>` and `ReadModel<P>` remain explicit

The public declaration API keeps both explicit role wrappers:

- `Automation<P>`
- `ReadModel<P>`

Internally, both normalize into `ConsumerSpec<P>`.

This keeps the slice-facing API intention-revealing while still preserving one shared runtime execution path. The runtime does not need duplicate automation-specific and read-model-specific pipelines. It only needs one normalized declaration with role and execution policy.

## Projector execution model after removing `DynProject`

The current runtime uses generic projector execution instead of the previous `DynProject` approach.

The declaration model did not need to change:

- `ConsumerSpec<P>` remains generic over `P`
- `Automation<P>` and `ReadModel<P>` remain generic over `P`
- sequential and concurrent execution still operate from the same declaration model

What changed is the execution boundary inside infrastructure.

### Why generic execution fits better

Generic execution is a better fit because the runtime actually needs only a few typed facts:

- the event names associated with `P::EventGroup`
- the ability to clone or own the projector value
- the ability to call `Project::project` after constructing a typed `Context`

Those requirements are a natural fit for ordinary generic bounds on `P`.

The old `DynProject` direction tried to erase:

- associated event group typing
- async projector invocation
- typed context creation

That pushed object safety and async dispatch concerns into a boundary that did not match the actual runtime domain. The runtime is not a general dynamic projector registry. It is a typed consumer runner that derives stream subjects from the projector event group and then invokes the projector for each typed message.

## Shared execution pipeline

The retained runtime design keeps one shared message-processing pipeline.

That shared path is responsible for:

- envelope conversion
- typed `Context` creation
- projector execution
- error mapping
- ack handling

Execution policy decides only how messages are scheduled:

- `Sequential`
- `Concurrent { max_in_flight }`

This means sequential and concurrent processing are still supported without changing the declaration API.

## Retained design decisions from the consumer declaration work

The current implementation keeps the design decisions established during the consumer declaration work:

- vertical slices declare consumers through explicit role wrappers
- infrastructure owns durable subscription creation
- infrastructure owns task spawning and lifecycle
- runtime execution is centralized in `NatsStore`
- one shared processing pipeline is reused for all declared consumers

The implementation also intentionally kept the wrapper-builder API and still skipped the intermediate convenience constructor approach on `ConsumerSpec`, so the main ergonomic entrypoints remain the explicit `Automation<P>` and `ReadModel<P>` builders.

## Practical usage shape

The intended usage remains:

- slices declare consumers in one place
- startup passes those declarations to `NatsStore`
- `NatsStore` derives the transport-specific behavior
- execution policy selects sequential or bounded concurrent processing

That preserves the declaration layer versus runtime layer split while keeping the projector execution model compile-safe and aligned with the actual runtime boundary.

