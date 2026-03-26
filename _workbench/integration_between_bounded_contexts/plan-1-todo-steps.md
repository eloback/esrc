## Step - Scope NATS command service name and subjects with bounded context prefix
      status: not_started
time-created: 2026-03-26 18:21:33

Currently, `CommandService::serve` (in `src/nats/command_service.rs`) registers the NATS micro service using only `event_name` (i.e., `A::Event::name()`) for:

- The service name passed to `.start(event_name, ...)`
- The group name passed to `.group(event_name)`
- The endpoint subject `command.*`

Similarly, `CommandClient::send_command` builds the request subject as `<event_name>.command.<id>`.

This means if two bounded contexts (each with their own `NatsStore` and prefix) happen to define aggregates whose events share the same `Event::name()`, the NATS service registrations and request subjects will collide.

### Changes required

- In `CommandService::serve` for `NatsStore`:
  - Use `self.prefix` (the bounded context name) to scope the NATS service name, e.g., `format!("{prefix}.{event_name}")` or similar.
  - Use the same scoped name for the group.
  - The endpoint subject should be scoped accordingly, e.g., `command.*` under the scoped group, or the full subject pattern should include the prefix.

- In `CommandClient::send_command` for `NatsStore`:
  - Update the request subject to include `self.prefix`, e.g., `format!("{prefix}.{event_name}.command.{id}")`.

- In `NatsStore::handle_request`:
  - Ensure the subject parsing (UUID extraction from the last token) still works correctly with the updated subject format.

- In `NatsStore::spawn_service`:
  - No structural changes needed beyond what `serve` already handles, but verify the log messages include the bounded context for clarity.

### Verification

- The `examples/operations/main.rs` example already uses a single bounded context (`BOUNDED_CONTEXT_NAME`). After the change, the subjects will be longer but functionally equivalent for single-context setups.
- The key improvement is that a multi-bounded-context application (multiple `NatsStore` instances with different prefixes) will have isolated command service namespaces.

### Notes

- The `CommandService` and `CommandClient` traits in `src/event/command_service.rs` do not need changes; they are transport-agnostic. Only the NATS implementation needs updating.
- The `self.prefix` field is already available on `NatsStore` and is the bounded context name.
