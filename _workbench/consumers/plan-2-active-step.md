## Step - implement the event_modeling module with declaration types and builders
      status: active
time-created: 2026-03-26 06:00:43
time-current: 2026-03-26 06:12:05

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

### Implementation Considerations
- Implemented the declaration-focused `src/event_modeling.rs` module and exposed it from `src/lib.rs`.
- Added the initial declaration types:
  - `ConsumerRole`
  - `ExecutionPolicy`
  - `ConsumerName`
  - `ConsumerSpec<P>`
- Added explicit slice-facing builders:
  - `Automation<P>`
  - `ReadModel<P>`
- Kept the implementation focused on declaration and normalization primitives only, with no runtime wiring in this step.
- Preserved the structured naming model through durable name and slice path helpers derived from `bounded_context`, `domain`, `feature`, and `consumer`.
