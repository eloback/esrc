# Skill, Create automations and read models with esrc event_modeling

## Goal

Declare event driven consumers for a vertical slice using `esrc::event_modeling` so that infrastructure can:

- Assign stable durable consumer names.
- Apply appropriate execution defaults based on consumer role.
- Run consumers sequentially or concurrently depending on the workload.
- Keep vertical slices isolated while still reading from shared event streams.

This skill focuses on the declaration side, not the infrastructure wiring.

## Key types

From `esrc::event_modeling`:

- `ConsumerName`
- `ConsumerRole`
- `ExecutionPolicy`
- `ConsumerSpec`
- `Automation`
- `ReadModel`

From `esrc::project`:

- `Project` trait, used by all consumers (automations and read models).

## Consumer naming standard

Use `ConsumerName::new(bounded_context, domain, feature, consumer)` with stable segments:

- `bounded_context`, use the bounded context constant.
- `domain`, use the domain constant.
- `feature`, use the slice `FEATURE_NAME`.
- `consumer`, a stable identifier for the particular consumer inside the slice.

Durable consumer name:

- The durable name is derived as: `<bounded_context>_<domain>_<feature>_<consumer>`
- Changing any segment effectively creates a new consumer, you lose the old consumer state and will reprocess from the start depending on backend behavior.

## Choosing automation vs read model

### Automation

Use an automation when:

- The consumer triggers side effects, workflows, or commands.
- The behavior is "reactive orchestration".
- Concurrency can improve throughput.

Default execution policy:

- `ConsumerRole::Automation` defaults to `ExecutionPolicy::Concurrent { max_in_flight: 16 }`.

### Read model

Use a read model consumer when:

- The consumer materializes queryable state.
- You need deterministic ordering or need to apply events sequentially per consumer.
- Writes to a view store must preserve event order.

Default execution policy:

- `ConsumerRole::ReadModel` defaults to `ExecutionPolicy::Sequential`.

Override guidance:

- Keep read models sequential unless you can guarantee correctness under concurrency.
- If you override automation concurrency, ensure `max_in_flight > 0`.

## Declaring a consumer in a slice

### Step 1, create a projector type

A consumer is ultimately a `Project` implementation. The `Project` receives a `Context<Envelope, EventGroup>` and processes it.

Requirements:

- `Project` is `Sync + Clone`.
- You define:
  - `type EventGroup`, an event type or an event group type, it must implement `EventGroup` and `TryFromEnvelope`.
  - `type Error`, errors stop processing and should be operationally meaningful.

Implementation guidance:

- Treat `Context` as a typed wrapper over an envelope, it prevents multiple deserializations.
- Use `Context` methods to access metadata, id, sequence, timestamp.
- Do not mix multiple vertical slices, the projector should only depend on the bounded context domain types, and slice local code.

### Step 2, create a structured name

In the slice root module:

- `const FEATURE_NAME: &str = "...";`

Then:

- `let name = ConsumerName::new(BOUNDED_CONTEXT_NAME, DOMAIN_NAME, FEATURE_NAME, "some_consumer");`

Choose the consumer segment:

- Use a short stable identifier:
  - `materialize`
  - `index`
  - `notify`
  - `dispatch`
  - `sync`
- If you have multiple consumers of the same role, use distinct consumer names.

### Step 3, declare Automation or ReadModel

Automation declaration:

- `let automation = Automation::new(name, projector);`

Read model declaration:

- `let read_model = ReadModel::new(name, projector);`

Override execution policy:

- `automation.max_concurrency(64)`
- or `with_execution_policy(ExecutionPolicy::Concurrent { max_in_flight: N })`
- or `read_model.with_execution_policy(ExecutionPolicy::Sequential)`

## Running consumers

The declaration results in an object that can be converted to a `ConsumerSpec`:

- `automation.into_spec()`
- `read_model.into_spec()`

Infrastructure then runs the spec:

- For example, a NATS backed store can run and spawn consumers by taking `ConsumerSpec<P>` and attaching it to the store task tracker.

Operational guidance:

- Prefer spawning consumers through infrastructure owned lifecycle management, rather than having slices spawn detached tasks.
- Keep projector state cloneable, but avoid holding heavy resources in it unless you are sure clones are safe and cheap.

## Testing approach

- Unit test projector logic by calling `project` with a constructed `Context`.
- Integration test the end to end flow by running the infrastructure consumer and publishing events.

When writing tests, focus on:

- Correct event pattern matching.
- Correct handling of envelope metadata.
- Idempotency if the backend can deliver duplicates.
