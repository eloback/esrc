## Step - Scope NATS command service name and subjects with bounded context prefix
      status: active
time-created: 2026-03-26 18:21:33
time-current: 2026-03-26 18:26:38

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

### Implementation Summary

All changes applied to `src/nats/command_service.rs`:

- `CommandService::serve`: introduced `scoped_name = format!("{prefix}.{event_name}")` used for the NATS micro service name (`.start`), description, and group name. The endpoint pattern `command.*` remains relative to the scoped group, so the full subject becomes `<prefix>.<event_name>.command.<id>`.

- `CommandClient::send_command`: updated request subject from `{event_name}.command.{id}` to `{prefix}.{event_name}.command.{id}`.

- `NatsStore::handle_request`: no changes needed; it extracts the UUID from the last subject token via `rsplit('.').next()`, which works regardless of how many prefix tokens precede it.

- `NatsStore::spawn_service`: updated log messages to include `prefix` for clarity.
