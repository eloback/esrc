====
>   Info: Duration: 32s | ~$0.1099 | Prompt Tokens: 34,854 (cached: 5,888) | Completion Tokens: 2,404 
>  Model: gpt-5.4 (gpt-5.4-2026-03-05) | Adapter: OpenAIResp
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git add -A . && git commit -m ". consumers - finalize active projector workflow validation step"
</suggested_git_command>

<FILE_CHANGES>
<FILE_NEW file_path="docs/event-modeling.md">
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

</FILE_NEW>

<FILE_APPEND file_path="_workbench/consumers/dev-chat.md">

## Request: Validate the generic projector runtime after removing DynProject

### Summary

The current generic projector runtime looks like the correct long-term direction.

What matters most is that removing `DynProject` did not force any declaration-layer redesign. The consumer declaration model still fits the runtime cleanly.

The retained shape is coherent:

- `ConsumerSpec<P>` is still the normalized declaration
- `Automation<P>` and `ReadModel<P>` are still the slice-facing role wrappers
- `ConsumerName` still provides stable durable identity
- `P::EventGroup` still drives subject derivation
- execution policy still chooses sequential or bounded concurrent processing

### Why the generic execution boundary is better

The runtime does not actually need a dynamically erased projector boundary.

It needs to know:

- which event group the projector consumes
- how to construct a typed `Context`
- how to call `Project::project`
- how to do that under the declared execution policy

Those are all ordinary generic concerns.

`DynProject` tried to erase the wrong boundary. It bundled:

- associated event group typing
- typed context creation
- async projector invocation

into one erased abstraction, but that abstraction was not the real runtime seam.

The real seam is simpler:

- declaration stays generic
- infrastructure runs the typed projector directly

That is why the current model is a better fit for the consumer runtime domain.

### What should be documented clearly

The workflow should now be documented around these retained decisions:

- declaration layer versus runtime layer remains the core split
- explicit `Automation<P>` and `ReadModel<P>` wrappers still matter for ergonomics
- `NatsStore` owns runtime execution and lifecycle
- one shared processing pipeline is used for envelope conversion, context creation, projector execution, error mapping, and acking
- the implementation intentionally kept the wrapper-builder API and still skipped the intermediate `ConsumerSpec` convenience constructor path

### Final view

This is a good outcome.

You kept the higher-level consumer declaration model that makes slice code readable, while simplifying the runtime to the typed behavior it actually needs. That should make the code easier to maintain and easier to extend later if you add retries, mirrors, or more execution policies.

</FILE_APPEND>

<FILE_PATCH file_path="_workbench/consumers/plan-2-active-step.md">
@@
-      status: active
+      status: done
 time-created: 2026-03-26 07:00:52
-time-current: 2026-03-26 07:10:03
+   time-done: 2026-03-26 07:12:21
@@
-References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.
@@
-### sub-step - validate the generic projector execution model and document the workflow
-time-current: 2026-03-26 07:10:03
-
-#### User ask
-
-following the plan rules, implement the next step of the plan.
-
-#### AI Answer
-
-Validated the retained consumer declaration integration after the generic projector runtime change and documented the final workflow.
-
-- Added `docs/event-modeling.md` as a focused guide describing:
-  - declaration layer versus runtime layer
-  - the retained role of `ConsumerSpec<P>`, `Automation<P>`, and `ReadModel<P>`
-  - the structured naming model
-  - why generic projector execution replaced `DynProject`
-  - how sequential and concurrent consumer execution remain supported
-
-- Appended a new dev chat entry capturing the retained design decisions and why the generic execution boundary better matches the runtime domain than the previous erased projector approach.
-
-- Kept the documentation aligned with the existing event modeling workflow and the earlier consumer declaration design decisions.
-
-
+### Summary
+- Validated that the retained declaration model still integrates with the NATS runtime after removing `DynProject`, including `ConsumerSpec<P>`, `Automation<P>`, `ReadModel<P>`, and the sequential and concurrent execution paths.
+- Added `docs/event-modeling.md` as a focused workflow guide covering the declaration layer versus runtime layer split, the structured naming model, and the retained role of the explicit declaration wrappers.
+- Documented why generic projector execution replaced `DynProject`, specifically that the runtime only needs typed event-group derivation, typed `Context` creation, and direct `Project::project` invocation.
+- Appended a new dev chat entry capturing the retained design decisions and clarifying why the generic execution boundary better matches the actual consumer runtime domain.
</FILE_PATCH>

<FILE_APPEND file_path="_workbench/consumers/plan-3-done-steps.md">

## Step - validate consumer declaration integration and document the updated projector model
      status: done
time-created: 2026-03-26 07:00:52
   time-done: 2026-03-26 07:12:21

Validate that the event modeling declarations still integrate correctly with the NATS runtime after the projector abstraction change, and document the retained design decisions.

- Verify the updated execution model still supports:
  - `ConsumerSpec<P>`
  - `Automation<P>`
  - `ReadModel<P>`
  - sequential and concurrent consumer execution paths

- Document the final projector execution approach, including why it replaced the previous `DynProject` design and how it better fits the consumer runtime domain.

- Keep the documentation aligned with the declaration layer versus runtime layer split already established in the event modeling workflow.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.

### Summary
- Validated that the retained declaration model still integrates with the NATS runtime after removing `DynProject`, including `ConsumerSpec<P>`, `Automation<P>`, `ReadModel<P>`, and the sequential and concurrent execution paths.
- Added `docs/event-modeling.md` as a focused workflow guide covering the declaration layer versus runtime layer split, the structured naming model, and the retained role of the explicit declaration wrappers.
- Documented why generic projector execution replaced `DynProject`, specifically that the runtime only needs typed event-group derivation, typed `Context` creation, and direct `Project::project` invocation.
- Appended a new dev chat entry capturing the retained design decisions and clarifying why the generic execution boundary better matches the actual consumer runtime domain.
</FILE_APPEND>
</FILE_CHANGES>

