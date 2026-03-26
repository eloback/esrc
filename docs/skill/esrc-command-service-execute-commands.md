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
