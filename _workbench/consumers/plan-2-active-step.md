## Step - implement the new projector execution abstraction and remove DynProject compile errors
      status: active
time-created: 2026-03-26 07:00:52
time-current: 2026-03-26 07:07:19

Replace the current `DynProject` implementation with the compile-safe abstraction defined in the previous step, updating the relevant runtime and declaration code so the crate builds correctly again.

- Update the projector execution flow used by consumer declarations and `NatsStore` runtime helpers.

- Ensure the implementation preserves the existing declaration-layer ergonomics where possible, while fixing object-safety, associated type, and async dispatch problems in the current design.

- Remove or adapt the existing `DynProject` machinery only as needed to support the new execution model cleanly.

References: see the retained event modeling design context in `plan-3-done-steps.md`, step `Step - integrate event_modeling declarations with NatsStore consumer startup`.

### Implementation Considerations

- Chosen direction: replace the current `DynProject` trait-object approach with a compile-safe consumer execution abstraction centered on a cloneable, generic runner input rather than dynamic projector dispatch.

- Runtime model:
  - Keep `ConsumerSpec<P>`, `Automation<P>`, and `ReadModel<P>` generic over the concrete projector type `P`.
  - Remove the need for `Box<dyn DynProject>` during message execution.
  - Let `NatsStore::run_consumer`, sequential execution, and concurrent execution remain generic over `P`, using ordinary monomorphized `Project` calls.

- Why the current `DynProject` direction should be replaced:
  - It mixes object safety, associated types, and typed `Context` construction in one erased trait surface.
  - `project_boxed` depends on `Self::EventGroup`, but `DynProject` does not define that associated type, so the abstraction does not match the execution boundary cleanly.
  - The runtime only needs a small subset of projector capabilities:
    - obtain event group names at startup
    - clone projector state when needed for message handling
    - execute the typed `Project` implementation for an incoming envelope
  - That is better represented through generic bounds on `P` than through a general-purpose trait object.

- Replacement abstraction responsibilities:
  - Startup metadata:
    - derive event subjects from `<P as Project>::EventGroup::names()`
    - keep this available through generic bounds requiring `P::EventGroup: EventGroup`
  - Message execution:
    - build `Context::<E, P::EventGroup>::try_with_envelope(envelope)` inside generic runtime helpers
    - invoke `Project::project` on a cloned or mutable projector value, depending on execution mode
  - Clone behavior:
    - sequential mode can own a single mutable projector instance for the lifetime of the consumer loop
    - concurrent mode should clone the projector per in-flight task, preserving the existing declaration-layer ergonomics
  - Async compatibility:
    - rely on the existing async `Project` trait directly
    - avoid erased async methods and pinned boxed futures in the runtime abstraction layer unless a future step explicitly introduces a dedicated factory or executor type

- Boxed projector values were evaluated but are not the chosen direction:
  - `Box<P>` is still generic over `P`, so it does not solve the erased dispatch problem by itself.
  - `Box<dyn DynProject>` reintroduces the same object-safety mismatch.
  - A boxed factory trait could work, but it would add indirection before there is a demonstrated need for heterogeneous consumer collections at the runtime boundary.

- If heterogeneous consumer registration is needed later, introduce a new erased type specifically for startup registration, not for per-message typed projector execution.
  - That future abstraction should own only:
    - durable consumer identity
    - event subject discovery
    - task spawning entrypoint
  - It should not attempt to erase the typed `Context` to run `Project` itself.

- Constraints for the next implementation step:
  - Remove `DynProject` from the runtime path used by `event_modeling` and `NatsStore`.
  - Update `ConsumerSpec<P>` and related builders so they only require projector capabilities where they are actually consumed, avoiding unnecessary trait bounds at declaration construction time.
  - Keep the declaration-layer API shape stable where possible.
  - Preserve support for:
    - sequential consumers with one mutable projector instance
    - concurrent consumers with bounded in-flight projector clones
    - subject derivation from the declared event group
    - typed envelope-to-context conversion through `TryFromEnvelope`

- Expected code direction in the next step:
  - `event_modeling` should depend on the declaration shape only, not on a dynamic projector trait.
  - `NatsStore` should become the owner of the generic execution logic for `P: Project + Clone + Send + Sync + 'static`.
  - `project.rs` should retain `Context` and `Project`, while removing or adapting only the `DynProject` machinery that caused the compile-safety issues.

### sub-step - replace dynproject runtime usage with generic projector execution
time-current: 2026-03-26 07:07:19

#### User ask

following the plan rules, implement the next step of the plan.

#### AI Answer

Implemented the runtime shift away from `DynProject` by making the NATS consumer execution path generic over concrete `Project` types.

- Removed the `DynProject` dependency from declaration construction in `event_modeling`.

- Removed the `DynProject` trait machinery from `project.rs`, while preserving `Context` and `Project`.

- Updated `NatsStore` startup and consumer execution helpers to:
  - derive event names from `P::EventGroup`
  - keep a single mutable projector for sequential execution
  - clone a projector per in-flight task for concurrent execution
  - build typed `Context` values directly from `NatsEnvelope`

- Preserved the declaration-layer API shape of `ConsumerSpec<P>`, `Automation<P>`, and `ReadModel<P>`, while moving execution concerns fully into generic runtime helpers.

