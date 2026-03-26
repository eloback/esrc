## Step - define the event_modeling module surface and consumer declaration model
      status: done
time-created: 2026-03-26 06:00:43
   time-done: 2026-03-26 06:09:46

Create the new `event_modeling` module plan and define the initial public API surface for consumer declarations. This step should introduce the core declaration concepts discussed in the dev chat, with explicit support for consumer roles such as automation and read model, and stable slice-oriented naming based on bounded context, domain, and feature.

- Define the planned types and responsibilities for:
  - consumer identity and naming
  - automation and read model declaration builders
  - a normalized internal consumer specification
  - execution policy defaults and overrides

- Capture the intended naming model so durable identities are derived from structured slice information instead of ad hoc strings.

- Favor the direction described in `_workbench/consumers/dev-chat.md`, request `Expand the consumer declaration design with code examples`, especially step 5 of `My recommended practical path`, while intentionally skipping step 4 for now.

### Summary
- Defined the `event_modeling` module surface as a declaration-focused crate API to be exposed from `src/lib.rs`, without runtime coupling in this step.
- Captured the core planned types:
  - `ConsumerRole`
  - `ExecutionPolicy`
  - `ConsumerName`
  - `ConsumerSpec<P>`
- Captured the slice-facing builder direction with explicit `Automation<P>` and `ReadModel<P>` wrappers that normalize into `ConsumerSpec<P>`.
- Recorded the structured durable naming model based on `bounded_context.domain.feature.consumer`.
- Clarified the runtime boundary for later steps, preserving infrastructure ownership of subscription creation, shutdown, acking, and shared message processing.
- Kept the sequencing aligned with the dev chat guidance by defining the surface first and deferring the later constructor-focused step as requested.

## Step - implement the event_modeling module with declaration types and builders
      status: done
time-created: 2026-03-26 06:00:43
   time-done: 2026-03-26 06:12:05

Add the new `event_modeling` module and implement the declaration-facing types needed by vertical slices.

- Create the module files and expose them from the crate.
- Implement the declaration model planned in the previous step, including:
  - consumer role representation
  - execution policy representation
  - structured consumer naming using bounded context, domain, and feature
  - normalized consumer specification
  - ergonomic `Automation` and `ReadModel` builders

- Keep the implementation focused on declaration and normalization primitives, without wiring runtime execution yet.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.

### Summary
- Added the new `event_modeling` module in `src/event_modeling.rs` and exposed it from `src/lib.rs`.
- Implemented declaration-focused consumer primitives:
  - `ConsumerRole`
  - `ExecutionPolicy`
  - `ConsumerName`
  - `ConsumerSpec<P>`
- Implemented explicit `Automation<P>` and `ReadModel<P>` builders that normalize into `ConsumerSpec<P>`.
- Added structured naming helpers for both stable durable consumer names and slice path representation.
- Kept the step scoped to declaration and normalization primitives, with no runtime startup integration yet.

## Step - integrate event_modeling declarations with NatsStore consumer startup
      status: done
time-created: 2026-03-26 06:00:43
   time-done: 2026-03-26 06:15:32

Implement a runtime entrypoint on `NatsStore` that accepts the new event modeling consumer specification and executes it using the existing `Project` pipeline.

- Add a shared consumer startup entrypoint that resolves the durable name from the structured declaration.
- Keep durable subscription creation as an infrastructure detail.
- Reuse a single message-processing pipeline for envelope conversion, typed context creation, projector execution, error mapping, and ack handling.

- Support execution policies for:
  - sequential processing
  - concurrent processing with bounded in-flight work

- Preserve infrastructure ownership of lifecycle concerns such as subscription creation and graceful shutdown wiring.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.

### Summary
- Added `NatsStore::run_consumer` as the runtime entrypoint that accepts `event_modeling::ConsumerSpec<P>`.
- Kept durable subscription setup inside infrastructure by deriving durable names and event subjects from the structured declaration and projector event group.
- Reused a shared message-processing pipeline for dynamic typed context creation, projector execution, error mapping, and ack handling.
- Implemented execution policy support for sequential and bounded concurrent processing.
- Added runtime validation for invalid concurrent configuration with a dedicated configuration error variant.
- Introduced erased projector execution support so declared consumer specs can be executed generically by the NATS runtime layer.

## Step - expose ergonomic spawning helpers for automation and read model consumers
      status: done
time-created: 2026-03-26 06:00:43
   time-done: 2026-03-26 06:21:27

Add high-level helpers that make startup code concise while keeping runtime ownership in infrastructure.

- Provide ergonomic methods or patterns for launching declared consumers with `NatsStore`.
- Ensure the API keeps vertical slice code focused on declaration intent rather than transport details.
- Align defaults with the semantics discussed in the dev chat:
  - automations favor concurrent execution
  - read models favor sequential execution

- Keep the helpers compatible with the structured naming model based on bounded context, domain, and feature.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.

### Summary
- Added `NatsStore` spawning helpers that launch declared consumers through the existing task tracker and runtime pipeline.
- Added helper entrypoints for normalized `ConsumerSpec` values and for slice-facing `Automation` and `ReadModel` declarations.
- Kept runtime ownership inside infrastructure by delegating execution to `run_consumer` and handling background task error logging within `NatsStore`.
- Preserved the existing structured naming and execution policy behavior because the spawning helpers reuse the declaration model without redefining transport concerns.

## Step - add a multi-slice example for command services and automation chaining
      status: done
time-created: 2026-03-26 06:07:23
   time-done: 2026-03-26 06:25:01

Create an example that declares at least two vertical slices and demonstrates the intended end-to-end workflow for the new consumer declaration model.

- The example should:
  - declare at least two slices
  - start the `CommandService`
  - execute at least one command manually
  - have automations listen to the published event stream and trigger new commands
  - keep running until ctrl-c is pressed

- Keep the example aligned with the declaration layer and runtime layer split established by the earlier `event_modeling` steps, so the example validates the intended slice-facing ergonomics rather than exposing transport wiring directly.

- Ensure the example is sequenced after the runtime and spawning helper work, since it depends on those pieces being available and should serve as a realistic usage reference.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.

### Summary
- Added `examples/multi-slice-command-service.rs` as an end-to-end reference for the event modeling workflow.
- Declared two slices, signup and email delivery, each with its own aggregate, commands, and events.
- Started both NATS-backed command services and sent an initial signup command manually through the command client API.
- Added automation consumers that react to published events and trigger follow-up commands across slices, demonstrating command chaining through the shared event stream.
- Kept the example running until ctrl-c so it matches the intended startup and background execution model for real applications.

## Step - document the event_modeling workflow and dev chat decisions
      status: done
time-created: 2026-03-26 06:00:43
   time-done: 2026-03-26 06:32:18

Document how vertical slices should declare consumers with the new `event_modeling` module, and record the key decisions carried forward from the dev chat.

- Explain the distinction between:
  - declaration layer owned by slices
  - runtime layer owned by infrastructure

- Document why automation and read model remain explicit concepts while sharing a normalized internal consumer specification.
- Document the structured naming approach using bounded context, domain, and feature.
- Summarize the intentional implementation choices taken from `_workbench/consumers/dev-chat.md`, including favoring step 5 of `My recommended practical path` and skipping step 4 for the initial implementation.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.

### Summary
- Added `docs/event-modeling.md` as a focused guide for the new declaration workflow and its declaration-layer versus runtime-layer split.
- Documented why `Automation<P>` and `ReadModel<P>` remain explicit slice-facing concepts while normalizing internally to `ConsumerSpec<P>`.
- Captured the structured naming model based on bounded context, domain, feature, and consumer segments.
- Appended a new dev chat entry recording that the implementation follows the recommended wrapper-builder path and intentionally skipped the initial `ConsumerSpec` convenience constructor step.
