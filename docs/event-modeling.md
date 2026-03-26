# Event modeling consumer workflow

This document explains how vertical slices should declare consumers with the `event_modeling` module and how infrastructure should execute them.

## Declaration layer vs runtime layer

The design is intentionally split into two layers.

### Declaration layer

Owned by vertical slices.

Slices should declare:

- which projector handles the events
- whether the consumer is an automation or a read model
- the stable structured consumer name
- any execution policy override that differs from the role default

This is the layer exposed by `src/event_modeling.rs`, mainly through:

- `ConsumerName`
- `ConsumerSpec<P>`
- `Automation<P>`
- `ReadModel<P>`

The slice should express intent, not transport mechanics.

### Runtime layer

Owned by infrastructure.

Infrastructure should handle:

- deriving durable consumer identities from declarations
- creating NATS durable consumers
- resolving event subjects from the projector event group
- running sequential or concurrent execution loops
- processing envelopes through the shared `Project` pipeline
- background spawning and lifecycle ownership

This is currently represented by `NatsStore::run_consumer`, `NatsStore::spawn_consumer`, `NatsStore::spawn_automation`, and `NatsStore::spawn_read_model`.

The runtime owns transport and lifecycle details so vertical slices do not need to wire shutdown, subscriptions, or loop mechanics directly.

## Why automation and read model stay explicit

The design keeps `Automation<P>` and `ReadModel<P>` as explicit slice-facing concepts even though both normalize into `ConsumerSpec<P>` internally.

That separation is intentional because the roles have different semantics:

- automations represent workflow-style reactions, often triggering commands, external calls, or side effects
- read models represent state-building consumers, usually favoring deterministic sequential processing

Keeping the two concepts explicit improves slice ergonomics and makes declarations more intention-revealing.

At the same time, both normalize into one internal specification so infrastructure can reuse a shared runtime path instead of duplicating startup and processing logic.

This gives the best of both directions:

- explicit role semantics for slice authors
- one normalized execution model for infrastructure

## Structured naming model

Consumer identities use a structured naming model through `ConsumerName`.

The naming segments are:

- bounded context
- domain
- feature
- consumer

This produces a stable durable consumer name shaped like:

- `bounded-context.domain.feature.consumer`

In the current implementation, `ConsumerName::durable_name()` formats the full durable identifier from these segments, and `ConsumerName::slice_path()` returns the path without the final consumer segment.

This structured naming was chosen to avoid ad hoc string arguments at call sites and to make durable consumers operationally understandable.

## Workflow for vertical slices

A typical slice-level workflow should look like this:

- create a `ConsumerName` with bounded context, domain, feature, and consumer segments
- choose `Automation::new(...)` or `ReadModel::new(...)`
- optionally override execution policy
- hand the declaration to `NatsStore` startup helpers

Conceptually:

- slices declare consumer intent
- startup registers or spawns declarations
- infrastructure executes them through the shared runtime pipeline

That keeps the feature-facing API focused on business meaning instead of NATS details.

## Decisions carried forward from the dev chat

The implementation follows the direction captured in `_workbench/consumers/dev-chat.md`.

### Main decisions retained

- durable subscription creation remains an infrastructure detail
- one normalized consumer specification is used internally
- automation and read model remain explicit declaration concepts
- execution behavior is represented as policy, not hard-coded role-specific loops at the API boundary
- startup ergonomics are provided through high-level spawning helpers on `NatsStore`

### Chosen practical path

The implementation intentionally favored step 5 from the dev chat section `My recommended practical path`:

- create `ConsumerSpec<P>`
- implement `run_consumer(spec)`
- centralize shared runtime execution
- add explicit wrapper builders
  - `Automation<P>`
  - `ReadModel<P>`

### Intentional skip

The implementation intentionally skipped step 4 for the initial version.

That means the crate does not currently expose convenience constructors directly on `ConsumerSpec`, such as:

- `ConsumerSpec::automation(...)`
- `ConsumerSpec::read_model(...)`

Instead, the initial public ergonomic layer is centered on the explicit wrapper builders:

- `Automation::new(...)`
- `ReadModel::new(...)`

This keeps the public surface aligned with the semantic distinction discussed in the dev chat while still normalizing internally to `ConsumerSpec<P>`.

## Current intended usage direction

For slice authors, the preferred shape is:

- declare read models with `ReadModel`
- declare automations with `Automation`
- keep NATS-specific setup in infrastructure
- use structured naming consistently

For infrastructure and startup code, the preferred shape is:

- accept normalized consumer declarations
- derive durable names and subjects from the declaration
- execute with shared runtime machinery
- preserve ownership of task lifecycle and background error reporting

That is the workflow the current implementation is optimizing for.

