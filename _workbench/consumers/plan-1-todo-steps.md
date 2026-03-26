
## Step - implement the new projector execution abstraction and remove DynProject compile errors
      status: not_started
time-created: 2026-03-26 07:00:52

Replace the current `DynProject` implementation with the compile-safe abstraction defined in the previous step, updating the relevant runtime and declaration code so the crate builds correctly again.

- Update the projector execution flow used by consumer declarations and `NatsStore` runtime helpers.

- Ensure the implementation preserves the existing declaration-layer ergonomics where possible, while fixing object-safety, associated type, and async dispatch problems in the current design.

- Remove or adapt the existing `DynProject` machinery only as needed to support the new execution model cleanly.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.

## Step - validate consumer declaration integration and document the updated projector model
      status: not_started
time-created: 2026-03-26 07:00:52

Validate that the event modeling declarations still integrate correctly with the NATS runtime after the projector abstraction change, and document the retained design decisions.

- Verify the updated execution model still supports:
  - `ConsumerSpec<P>`
  - `Automation<P>`
  - `ReadModel<P>`
  - sequential and concurrent consumer execution paths

- Document the final projector execution approach, including why it replaced the previous `DynProject` design and how it better fits the consumer runtime domain.

- Keep the documentation aligned with the declaration layer versus runtime layer split already established in the event modeling workflow.

References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.
