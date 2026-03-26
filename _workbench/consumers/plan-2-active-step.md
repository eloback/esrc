## Step - integrate event_modeling declarations with NatsStore consumer startup
      status: active
time-created: 2026-03-26 06:00:43
time-current: 2026-03-26 06:15:32

Implement a runtime entrypoint on `NatsStore` that accepts the new event modeling consumer specification and executes it using the existing `Project` pipeline.

- Add a shared consumer startup entrypoint that resolves the durable name from the structured declaration.
- Keep durable subscription creation as an infrastructure detail.
- Reuse a single message-processing pipeline for envelope conversion, typed context creation, projector execution, error mapping, and ack handling.

- Support execution policies for:
  - sequential processing
  - concurrent processing with bounded in-flight work

- Preserve infrastructure ownership of lifecycle concerns such as subscription creation and graceful shutdown wiring.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.

### Implementation Considerations
- Added `NatsStore::run_consumer` as the shared runtime entrypoint for `event_modeling::ConsumerSpec`.
- Kept durable consumer creation internal to `NatsStore`, deriving the durable name and subscribed subjects from the structured declaration and projector event group.
- Added a shared message-processing path that converts the envelope, builds typed projection context dynamically, runs the projector, maps user errors into crate `Error`, and acks successful messages.
- Implemented execution policy support for:
  - sequential processing that propagates failures
  - concurrent processing with bounded in-flight work and per-message error logging
- Added runtime validation for invalid concurrent configuration such as `max_in_flight == 0`.
- Introduced erased projector execution support so runtime startup can operate on declared consumer specs without exposing transport details to slices.
