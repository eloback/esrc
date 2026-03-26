====
>   Info: Duration: 1m | ~$0.1189 | Prompt Tokens: 25,378 (cached: 4,864) | Completion Tokens: 4,425 
>  Model: gpt-5.4 (gpt-5.4-2026-03-05) | Adapter: OpenAIResp
>  Files: Context Files: 10 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git commit -a -m "^ consumers - validate generic projector runtime and document retained execution model"
</suggested_git_command>

<FILE_CHANGES>
<FILE_NEW file_path="docs/event-modeling.md">
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
</FILE_NEW>

<FILE_APPEND file_path="_workbench/consumers/dev-chat.md">

## Request: Validate the updated projector model after removing DynProject

### Summary

The generic projector execution model is the right long-term fit for the current consumer runtime.

The important result is that the declaration model did not need to change in order to remove `DynProject`.

What changed is only the execution boundary inside infrastructure:

- declaration types remain generic over `P`
- `NatsStore` now owns the typed execution path directly
- sequential and concurrent execution still work with the same declaration API

### What was validated

The retained integration points are still coherent:

- `ConsumerSpec<P>` remains the normalized declaration type
- `Automation<P>` and `ReadModel<P>` still provide the intended slice-facing ergonomics
- durable naming still comes from `ConsumerName`
- subject derivation still comes from `P::EventGroup`
- execution policy still selects between sequential and bounded concurrent processing

That means the original declaration-layer versus runtime-layer split still holds after the projector abstraction change.

### Why the generic projector model is better than `DynProject`

Replacing `DynProject` with generic execution was the correct move because the runtime does not actually need per-message dynamic dispatch over an erased typed context boundary.

The runtime needs to know only:

- which event names belong to the projector event group
- how to own or clone the projector value
- how to invoke `Project::project` once a typed `Context` has been created

Those are all a natural fit for ordinary generic bounds on `P`.

The earlier `DynProject` direction pushed object safety, associated types, and async execution into one erased abstraction, but that abstraction did not correspond to the true runtime boundary.

### Retained design decisions to document

The final workflow should explicitly document that the implementation kept these earlier decisions:

- slice-facing declarations stay explicit through `Automation<P>` and `ReadModel<P>`
- runtime execution is centralized in `NatsStore`
- one shared processing pipeline is used for envelope conversion, context creation, projector execution, error mapping, and acking
- the implementation intentionally kept the wrapper-builder API and still skipped the intermediate `ConsumerSpec` convenience constructor step

### Documentation recommendation

The most useful documentation shape is a focused workflow guide that explains:

- declaration layer versus runtime layer
- why `Automation<P>` and `ReadModel<P>` are still explicit
- how structured naming works
- why generic projector execution replaced `DynProject`
- how sequential and concurrent execution remain supported without changing the declaration model

That would preserve the reasoning behind the current API shape and make the new execution model much easier to understand later.
</FILE_APPEND>

<FILE_PATCH file_path="_workbench/consumers/plan-1-todo-steps.md">
@@
-## Step - validate consumer declaration integration and document the updated projector model
-      status: not_started
-time-created: 2026-03-26 07:00:52
-
-Validate that the event modeling declarations still integrate correctly with the NATS runtime after the projector abstraction change, and document the retained design decisions.
-
-- Verify the updated execution model still supports:
-  - `ConsumerSpec<P>`
-  - `Automation<P>`
-  - `ReadModel<P>`
-  - sequential and concurrent consumer execution paths
-
-- Document the final projector execution approach, including why it replaced the previous `DynProject` design and how it better fits the consumer runtime domain.
-
-- Keep the documentation aligned with the declaration layer versus runtime layer split already established in the event modeling workflow.
-
-References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.
+
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-2-active-step.md">
@@
-## Step - implement the new projector execution abstraction and remove DynProject compile errors
-      status: active
-time-created: 2026-03-26 07:00:52
-time-current: 2026-03-26 07:07:19
-
-Replace the current `DynProject` implementation with the compile-safe abstraction defined in the previous step, updating the relevant runtime and declaration code so the crate builds correctly again.
-
-- Update the projector execution flow used by consumer declarations and `NatsStore` runtime helpers.
-
-- Ensure the implementation preserves the existing declaration-layer ergonomics where possible, while fixing object-safety, associated type, and async dispatch problems in the current design.
-
-- Remove or adapt the existing `DynProject` machinery only as needed to support the new execution model cleanly.
-
-References: see the retained event modeling design context in `plan-3-done-steps.md`, step `Step - integrate event_modeling declarations with NatsStore consumer startup`.
-
-### Implementation Considerations
-
-- Chosen direction: replace the current `DynProject` trait-object approach with a compile-safe consumer execution abstraction centered on a cloneable, generic runner input rather than dynamic projector dispatch.
-
-- Runtime model:
-  - Keep `ConsumerSpec<P>`, `Automation<P>`, and `ReadModel<P>` generic over the concrete projector type `P`.
-  - Remove the need for `Box<dyn DynProject>` during message execution.
-  - Let `NatsStore::run_consumer`, sequential execution, and concurrent execution remain generic over `P`, using ordinary monomorphized `Project` calls.
-
-- Why the current `DynProject` direction should be replaced:
-  - It mixes object safety, associated types, and typed `Context` construction in one erased trait surface.
-  - `project_boxed` depends on `Self::EventGroup`, but `DynProject` does not define that associated type, so the abstraction does not match the execution boundary cleanly.
-  - The runtime only needs a small subset of projector capabilities:
-    - obtain event group names at startup
-    - clone projector state when needed for message handling
-    - execute the typed `Project` implementation for an incoming envelope
-  - That is better represented through generic bounds on `P` than through a general-purpose trait object.
-
-- Replacement abstraction responsibilities:
-  - Startup metadata:
-    - derive event subjects from `<P as Project>::EventGroup::names()`
-    - keep this available through generic bounds requiring `P::EventGroup: EventGroup`
-  - Message execution:
-    - build `Context::<E, P::EventGroup>::try_with_envelope(envelope)` inside generic runtime helpers
-    - invoke `Project::project` on a cloned or mutable projector value, depending on execution mode
-  - Clone behavior:
-    - sequential mode can own a single mutable projector instance for the lifetime of the consumer loop
-    - concurrent mode should clone the projector per in-flight task, preserving the existing declaration-layer ergonomics
-  - Async compatibility:
-    - rely on the existing async `Project` trait directly
-    - avoid erased async methods and pinned boxed futures in the runtime abstraction layer unless a future step explicitly introduces a dedicated factory or executor type
-
-- Boxed projector values were evaluated but are not the chosen direction:
-  - `Box<P>` is still generic over `P`, so it does not solve the erased dispatch problem by itself.
-  - `Box<dyn DynProject>` reintroduces the same object-safety mismatch.
-  - A boxed factory trait could work, but it would add indirection before there is a demonstrated need for heterogeneous consumer collections at the runtime boundary.
-
-- If heterogeneous consumer registration is needed later, introduce a new erased type specifically for startup registration, not for per-message typed projector execution.
-  - That future abstraction should own only:
-    - durable consumer identity
-    - event subject discovery
-    - task spawning entrypoint
-  - It should not attempt to erase the typed `Context` to run `Project` itself.
-
-- Constraints for the next implementation step:
-  - Remove `DynProject` from the runtime path used by `event_modeling` and `NatsStore`.
-  - Update `ConsumerSpec<P>` and related builders so they only require projector capabilities where they are actually consumed, avoiding unnecessary trait bounds at declaration construction time.
-  - Keep the declaration-layer API shape stable where possible.
-  - Preserve support for:
-    - sequential consumers with one mutable projector instance
-    - concurrent consumers with bounded in-flight projector clones
-    - subject derivation from the declared event group
-    - typed envelope-to-context conversion through `TryFromEnvelope`
-
-- Expected code direction in the next step:
-  - `event_modeling` should depend on the declaration shape only, not on a dynamic projector trait.
-  - `NatsStore` should become the owner of the generic execution logic for `P: Project + Clone + Send + Sync + 'static`.
-  - `project.rs` should retain `Context` and `Project`, while removing or adapting only the `DynProject` machinery that caused the compile-safety issues.
-
-### sub-step - replace dynproject runtime usage with generic projector execution
-time-current: 2026-03-26 07:07:19
-
-#### User ask
-
-following the plan rules, implement the next step of the plan.
-
-#### AI Answer
-
-Implemented the runtime shift away from `DynProject` by making the NATS consumer execution path generic over concrete `Project` types.
-
-- Removed the `DynProject` dependency from declaration construction in `event_modeling`.
-
-- Removed the `DynProject` trait machinery from `project.rs`, while preserving `Context` and `Project`.
-
-- Updated `NatsStore` startup and consumer execution helpers to:
-  - derive event names from `P::EventGroup`
-  - keep a single mutable projector for sequential execution
-  - clone a projector per in-flight task for concurrent execution
-  - build typed `Context` values directly from `NatsEnvelope`
-
-- Preserved the declaration-layer API shape of `ConsumerSpec<P>`, `Automation<P>`, and `ReadModel<P>`, while moving execution concerns fully into generic runtime helpers.
+## Step - validate consumer declaration integration and document the updated projector model
+      status: active
+time-created: 2026-03-26 07:00:52
+time-current: 2026-03-26 07:10:03
+
+Validate that the event modeling declarations still integrate correctly with the NATS runtime after the projector abstraction change, and document the retained design decisions.
+
+- Verify the updated execution model still supports:
+  - `ConsumerSpec<P>`
+  - `Automation<P>`
+  - `ReadModel<P>`
+  - sequential and concurrent consumer execution paths
+
+- Document the final projector execution approach, including why it replaced the previous `DynProject` design and how it better fits the consumer runtime domain.
+
+- Keep the documentation aligned with the declaration layer versus runtime layer split already established in the event modeling workflow.
+
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.
+
+### sub-step - validate the generic projector execution model and document the workflow
+time-current: 2026-03-26 07:10:03
+
+#### User ask
+
+following the plan rules, implement the next step of the plan.
+
+#### AI Answer
+
+Validated the retained consumer declaration integration after the generic projector runtime change and documented the final workflow.
+
+- Added `docs/event-modeling.md` as a focused guide describing:
+  - declaration layer versus runtime layer
+  - the retained role of `ConsumerSpec<P>`, `Automation<P>`, and `ReadModel<P>`
+  - the structured naming model
+  - why generic projector execution replaced `DynProject`
+  - how sequential and concurrent consumer execution remain supported
+
+- Appended a new dev chat entry capturing the retained design decisions and why the generic execution boundary better matches the runtime domain than the previous erased projector approach.
+
+- Kept the documentation aligned with the existing event modeling workflow and the earlier consumer declaration design decisions.
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-3-done-steps.md">
@@
 ## Step - document the event_modeling workflow and dev chat decisions
       status: done
 time-created: 2026-03-26 06:00:43
    time-done: 2026-03-26 06:32:18
@@
 - Added `docs/event-modeling.md` as a focused guide for the new declaration workflow and its declaration-layer versus runtime-layer split.
 - Documented why `Automation<P>` and `ReadModel<P>` remain explicit slice-facing concepts while normalizing internally to `ConsumerSpec<P>`.
 - Captured the structured naming model based on bounded context, domain, feature, and consumer segments.
 - Appended a new dev chat entry recording that the implementation follows the recommended wrapper-builder path and intentionally skipped the initial `ConsumerSpec` convenience constructor step.
+
+## Step - implement the new projector execution abstraction and remove DynProject compile errors
+      status: done
+time-created: 2026-03-26 07:00:52
+   time-done: 2026-03-26 07:07:19
+
+Replace the current `DynProject` implementation with the compile-safe abstraction defined in the previous step, updating the relevant runtime and declaration code so the crate builds correctly again.
+
+- Update the projector execution flow used by consumer declarations and `NatsStore` runtime helpers.
+
+- Ensure the implementation preserves the existing declaration-layer ergonomics where possible, while fixing object-safety, associated type, and async dispatch problems in the current design.
+
+- Remove or adapt the existing `DynProject` machinery only as needed to support the new execution model cleanly.
+
+References: see the retained event modeling design context in `plan-3-done-steps.md`, step `Step - integrate event_modeling declarations with NatsStore consumer startup`.
+
+### Summary
+- Replaced the runtime `DynProject` path with compile-safe generic projector execution based on concrete `Project` types.
+- Kept `ConsumerSpec<P>`, `Automation<P>`, and `ReadModel<P>` as generic declaration-layer types without reintroducing erased projector dispatch into the runtime path.
+- Updated `NatsStore` consumer startup and execution helpers to derive subjects from `P::EventGroup`, create typed `Context` values directly, and execute sequential or concurrent projector handling through ordinary generic bounds.
+- Removed the failing dynamic projector machinery from `project.rs` while preserving the existing `Context` and `Project` surface.
</FILE_PATCH>
</FILE_CHANGES>



