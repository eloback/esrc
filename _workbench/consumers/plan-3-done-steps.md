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
