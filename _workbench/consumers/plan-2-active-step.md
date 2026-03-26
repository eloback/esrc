## Step - expose ergonomic spawning helpers for automation and read model consumers
      status: active
time-created: 2026-03-26 06:00:43
time-current: 2026-03-26 06:21:27

Add high-level helpers that make startup code concise while keeping runtime ownership in infrastructure.

- Provide ergonomic methods or patterns for launching declared consumers with `NatsStore`.
- Ensure the API keeps vertical slice code focused on declaration intent rather than transport details.
- Align defaults with the semantics discussed in the dev chat:
  - automations favor concurrent execution
  - read models favor sequential execution

- Keep the helpers compatible with the structured naming model based on bounded context, domain, and feature.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.

### Implementation Considerations
- Added spawning helpers on `NatsStore` that launch declared consumers onto the existing task tracker instead of requiring callers to wire `run_consumer` manually.
- Added helper variants for `ConsumerSpec`, `Automation`, and `ReadModel` so startup code can stay at the declaration layer without transport-specific conversion boilerplate.
- Preserved infrastructure ownership of lifecycle and error reporting by keeping task spawning and runtime failure logging inside `NatsStore`.
- Kept the helpers compatible with the existing structured consumer naming and execution policy defaults because they delegate to `run_consumer`.
