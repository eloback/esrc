## Step - add a multi-slice example for command services and automation chaining
      status: active
time-created: 2026-03-26 06:07:23
time-current: 2026-03-26 06:25:01

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

### Implementation Considerations
- Added a new `examples/multi-slice-command-service.rs` example that keeps the slice-facing startup flow at the declaration layer by using `Automation` declarations and `NatsStore` spawning helpers.
- Modeled two slices, signup and email delivery, with separate aggregate, command, and event types so the example shows cross-slice command chaining through published events.
- Started both command services, sent one manual signup command, and wired automations that react to the event stream by sending follow-up commands through the command client API.
- Kept the example process alive until ctrl-c so it serves as a realistic end-to-end reference for command services and background consumers running together.
