====
>   Info: Duration: 1m 5s | ~$0.0743 | Prompt Tokens: 13,282 | Completion Tokens: 3,647 
>  Model: gpt-5.2 (gpt-5.2-2025-12-11) | Adapter: OpenAI
>  Files: Context Files: 8 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git add -A .
git commit -m "+ docs - Add esrc vertical slice skill docs"
</suggested_git_command>

<FILE_CHANGES>

<FILE_NEW file_path="doc/skill/esrc-slice-constants-and-module-layout.md">
```markdown
# Skill, Define an esrc vertical slice layout and constants

## Goal

Standardize how a vertical slice is structured in an esrc based project, ensuring:

- The slice is independent and does not import code from other slices.
- The slice declares `FEATURE_NAME` as a `const` at the root of the slice.
- The domain exposes `BOUNDED_CONTEXT_NAME` and `DOMAIN_NAME` constants.
- Instrumentation and consumer naming can reuse stable, consistent segments.

## Assumptions

- The bounded context and domain modules exist and are available as context to the implementer.
- A vertical slice is a feature folder, for example `dashboard_of_operations`, `send_email_to_user`, `create_operation`.
- You cannot import code from other vertical slices.
- The slice can import from the bounded context domain modules and from infrastructure (esrc, nats store wiring) as allowed by the project.

## Required constants

### Bounded context constant

File:

- `<bounded_context_name>/domain/mod.rs`

Required:

- `BOUNDED_CONTEXT_NAME` must exist and be a stable identifier for the bounded context.

Example:

- `const BOUNDED_CONTEXT_NAME: &str = <bounded_context_name>;`

### Domain constant

File:

- `<bounded_context_name>/domain/<domain_name>/mod.rs`

Required:

- `DOMAIN_NAME` must exist and be a stable identifier for the domain.

Example:

- `const DOMAIN_NAME: &str = <domain_name>;`

### Slice feature constant

File:

- `<bounded_context_name>/<domain_name>/<slice_folder>/mod.rs` (or the root module for the slice)

Required:

- Declare `FEATURE_NAME` in the root of the slice.

Example:

- `const FEATURE_NAME: &str = "criar_batch";`

Guidelines:

- The value should be a stable feature identifier, this is used for instrumentation and configuration.
- Prefer lowercase, and keep it stable once deployed, changing it changes consumer durable names and monitoring identifiers.

## Suggested slice folder layout

A minimal, scalable layout:

- `<slice_folder>/`
  - `mod.rs`
  - `generated.rs`
  - `read_model.rs` (optional)
  - `automation.rs` (optional)
  - `commands.rs` (optional helper types for slice owned commands, if any)
  - `wiring.rs` (optional, used to expose a single entry point to attach consumers to a store)

Notes:

- `generated.rs` is the public interface for read models and queries, and is typically overwritten by code generation.
- `mod.rs` re-exports public types, and is the best place to expose custom queries that are not generated.
- Keep the slice code cohesive, avoid splitting into many files early unless it is needed.

## Public API rules for the slice

- Treat `generated.rs` as part of the slice public API, it should be stable and ergonomic.
- Use `mod.rs` to re-export the generated types and also add custom queries and public entry points.

Example public exports pattern:

- `pub use generated::*;`
- `pub use read_model::...;`
- `pub use automation::...;`

## Anti patterns

- Importing a read model, automation, or internal helper type from another slice.
- Hiding `FEATURE_NAME` inside a submodule, it must be at the root to be consistently discoverable.
- Reusing a single consumer durable name across multiple features, it breaks operational isolation.
- Using unstable identifiers derived from environment variables as the constants, constants must be stable at compile time.
```
</FILE_NEW>

<FILE_NEW file_path="doc/skill/esrc-event-modeling-create-consumers-automations-and-read-models.md">
```markdown
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
```
</FILE_NEW>

<FILE_NEW file_path="doc/skill/esrc-command-service-execute-commands.md">
```markdown
# Skill, Execute commands with esrc command_service

## Goal

Send commands to an aggregate command service using `esrc::event::command_service::CommandClient` so that:

- A slice can trigger state changes through aggregates.
- Errors are correctly mapped and handled.
- Command execution stays isolated within the bounded context domain, without cross slice dependencies.

This skill focuses on client side command execution from within a slice, and the server side expectations.

## Command execution model

In esrc, command handling follows:

- Aggregates implement `Aggregate`:
  - `type Command`
  - `type Event`
  - `type Error`
  - `process(&self, command) -> Result<Event, Error>`
  - `apply(self, event) -> Self`

A command service implementation is responsible for:

- Receiving a serialized command.
- Reconstructing aggregate state by replaying events.
- Calling `Aggregate::process`.
- Persisting the produced event with optimistic concurrency.
- Replying with success or error.

A command client is responsible for:

- Serializing `A::Command`.
- Routing to the service endpoint for aggregate `A` and `Uuid`.
- Decoding reply payload.
- Mapping failures to `crate::error::Error`.

## When a slice should send a command

A slice should send commands when:

- It is implementing an automation that reacts to events and needs to drive the system forward.
- It must coordinate a workflow across aggregates, in that case it still sends commands to the relevant aggregate types.

A slice should not send commands when:

- It can compute the projection it needs directly by consuming events, then it should build a read model.
- It would require importing code from another vertical slice, instead depend on the bounded context domain types and the shared interfaces.

## How to send a command

### Inputs

- Aggregate type `A`, known from bounded context domain.
- Aggregate id `Uuid`.
- Concrete command value `A::Command`.

### Requirements

- You have a type that implements `CommandClient`, for example a store client.

### Call pattern

- `client.send_command::<A>(id, command).await`

You handle results as `error::Result<()>`:

- `Ok(())`, the command was accepted and a resulting event was persisted.
- `Err(error::Error::Conflict)`, optimistic concurrency conflict, retry policy is context dependent.
- `Err(error::Error::External(_))`, the aggregate rejected the command with a domain error.
- `Err(error::Error::Internal(_))`, infrastructure failure, typically non recoverable without intervention.
- `Err(error::Error::Format(_))`, serialization or protocol mapping issues.

Guidelines:

- Only retry on `Conflict` if your command is safe to retry and you are sure you can reload and reissue it.
- External errors should be treated as domain outcomes, not infrastructure failures.
- Internal errors should be logged with enough context and typically surface to operational alerting.

## How a slice should structure command sending

### Keep the command sending at the edge of the slice

Prefer to:

- Keep a small function in the slice automation that performs:
  - event handling decisions
  - command construction
  - command sending
  - error mapping

Avoid:

- Threading a `CommandClient` through deep pure logic layers, keep pure decision making separate from IO.

### Always include feature naming context

Because `FEATURE_NAME` exists in the slice, use it consistently for:

- Tracing instrumentation spans, fields, and log tags.
- Consumer name segments if sending commands is part of a consumer.

Even if the command client API does not accept feature context directly, the slice should ensure that:

- any tracing spans created around command execution include the feature constant.

## Expected server side behavior

From the caller perspective, assume:

- The server loads aggregate state from event store.
- The server enforces optimistic concurrency.
- The server reply is deterministic given the aggregate state and command.

When a slice depends on this behavior, ensure:

- Commands are designed to be either idempotent, or safe under retries.
- Domain errors are serializable if the transport requires it.
- Domain errors remain stable, because they may be observed by clients.

## Suggested error handling mapping in slices

A common slice level mapping:

- `Conflict`:
  - retry with backoff if safe
  - or convert to a workflow state, for example "try again"
- `External(domain_error)`:
  - decide based on domain error:
    - ignore, compensate, or stop workflow
- `Internal`:
  - stop consumer and surface error, since continuing may hide systemic issues

Do not swallow internal errors in automations:

- In an automation consumer, returning an error should stop processing, preventing silent data loss.

## Testing approach

- Unit test command creation based on given event inputs.
- Integration test the command service endpoint, including:
  - correct subject routing
  - serialization roundtrip
  - concurrency conflict behavior

If the backend is NATS, integration tests should include:

- starting the service
- sending a command with `send_command`
- verifying that an event is persisted and can be replayed
```
</FILE_NEW>

<FILE_NEW file_path="doc/skill/esrc-read-model-public-interface-and-queries.md">
```markdown
# Skill, Define read model public interfaces and queries in a slice

## Goal

Define the public interfaces for slice read models and queries using a stable pattern:

- `generated.rs` holds generated read model structs and basic query structs.
- `mod.rs` re-exports generated types and declares any additional custom queries.
- Consumers and projectors can depend on these public types without importing from other slices.

## Required generated interface file

File:

- `<slice_folder>/generated.rs`

Required:

- It contains the read model data structures and basic query structures.
- It is considered public interface for the slice.
- It is codegen owned, the slice implementation must treat it as generated output.

Given example:

- It starts with `/// Code generated by v8digital.em-rust-code-gen.`
- It uses:
  - `serde::{Deserialize, Serialize}`
  - `uuid::Uuid`

Example shape:

- `pub struct Rede { ... }`
- `pub struct RedeQuery { ... }`

Guidelines:

- Keep read model structs:
  - `Debug, Clone, Default, PartialEq, Serialize, Deserialize` where applicable.
- Keep query structs:
  - `Debug, Serialize, Deserialize, Clone`
- Use `Option<T>` for optional filters.

## Exposing additional queries

When a slice needs a more complex query than what is generated:

- Declare it in `<slice_folder>/mod.rs` so it is exposed alongside generated queries.

Pattern:

- `pub enum Queries` groups all public query options for the slice.
- Include a `Default` variant that contains the generated query type.

Example pattern:

- `pub enum Queries { Default { query: RedeQuery }, ByMaster { query: RedeByMasterQuery } }`
- `pub struct RedeByMasterQuery { pub master_id: Uuid }`

Guidelines:

- Use `enum Queries` when:
  - you want a single query entry point in transport layers.
  - you want to keep query routing explicit.
- Use distinct query structs for clarity and stable evolution.
- Keep query types serializable because they are often used by http routes, message queues, or rpc layers.

## How read model code should use these types

Inside slice read model logic:

- Treat `generated.rs` as the schema boundary.
- Keep internal storage models separate when needed, but map to the public structs.

Common approaches:

- In memory view:
  - store a map keyed by `Uuid`, return `Rede` snapshots for queries.
- Persistent view:
  - store in database, query by fields based on `RedeQuery` or custom queries.

Avoid:

- Returning internal storage types directly, always return the public interface types.
- Having multiple competing public query entry points across files.

## Compatibility and evolution

Changing public structs and queries affects:

- serialization formats in external interfaces.
- compatibility with already persisted read models.
- client code.

Guidelines:

- Prefer additive changes:
  - add optional fields
  - add new query variants
- Avoid breaking changes unless coordinated:
  - renaming fields
  - changing types
  - removing query variants

## Testing approach

- Unit test query filtering logic using `RedeQuery` and any custom queries.
- If queries are used in transports, add contract tests for serialization and routing.
```
</FILE_NEW>

</FILE_CHANGES>

