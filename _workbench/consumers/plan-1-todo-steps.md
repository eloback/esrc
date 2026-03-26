## Step - add a multi-slice example for command services and automation chaining
      status: not_started
time-created: 2026-03-26 06:07:23

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

## Step - document the event_modeling workflow and dev chat decisions
      status: not_started
time-created: 2026-03-26 06:00:43

Document how vertical slices should declare consumers with the new `event_modeling` module, and record the key decisions carried forward from the dev chat.

- Explain the distinction between:
  - declaration layer owned by slices
  - runtime layer owned by infrastructure

- Document why automation and read model remain explicit concepts while sharing a normalized internal consumer specification.
- Document the structured naming approach using bounded context, domain, and feature.
- Summarize the intentional implementation choices taken from `_workbench/consumers/dev-chat.md`, including favoring step 5 of `My recommended practical path` and skipping step 4 for the initial implementation.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
