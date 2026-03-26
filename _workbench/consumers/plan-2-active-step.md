## Step - define the event_modeling module surface and consumer declaration model
      status: active
time-created: 2026-03-26 06:00:43
time-current: 2026-03-26 06:09:46

Create the new `event_modeling` module plan and define the initial public API surface for consumer declarations. This step should introduce the core declaration concepts discussed in the dev chat, with explicit support for consumer roles such as automation and read model, and stable slice-oriented naming based on bounded context, domain, and feature.

- Define the planned types and responsibilities for:
  - consumer identity and naming
  - automation and read model declaration builders
  - a normalized internal consumer specification
  - execution policy defaults and overrides

- Capture the intended naming model so durable identities are derived from structured slice information instead of ad hoc strings.

- Favor the direction described in `_workbench/consumers/dev-chat.md`, request `Expand the consumer declaration design with code examples`, especially step 5 of `My recommended practical path`, while intentionally skipping step 4 for now.

### Implementation Considerations
- Public crate surface:
  - add a new `event_modeling` module exposed from `src/lib.rs`
  - keep the initial API declaration-focused, with no runtime coupling in this step

- Planned declaration types:
  - `ConsumerRole`, with explicit `Automation` and `ReadModel` variants
  - `ExecutionPolicy`, with `Sequential` and bounded `Concurrent { max_in_flight }`
  - `ConsumerName`, carrying structured `bounded_context`, `domain`, and `feature` segments, plus a stable consumer identifier
  - `ConsumerSpec<P>`, as the normalized internal runtime-facing representation that holds identity, role, execution policy, and projector

- Planned slice-facing builders:
  - `Automation<P>` wrapping a normalized `ConsumerSpec<P>` and defaulting to concurrent execution
  - `ReadModel<P>` wrapping a normalized `ConsumerSpec<P>` and defaulting to sequential execution
  - both builders should normalize into `ConsumerSpec<P>` so runtime integration can later accept one shared shape

- Naming model:
  - durable consumer identities should be derived from structured slice information, not arbitrary strings
  - the naming shape should be stable and slice-oriented, using `bounded_context.domain.feature.consumer`
  - expose helpers that return the stable durable consumer name and a slice path representation, so startup and observability can share the same structured identity source

- Runtime boundary for later steps:
  - `NatsStore` should eventually accept `ConsumerSpec<P>` through a shared `run_consumer` entrypoint
  - durable subscription creation, graceful shutdown, acking, and message processing remain infrastructure-owned and should not leak into declaration builders in this step

- Intentional sequencing:
  - this step defines the module surface and responsibilities only
  - step 4 from `My recommended practical path` remains intentionally deferred for the initial implementation path, as requested by the todo step
