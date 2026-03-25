## Step - Add CommandError response type and command_service module skeleton
      status: done
time-created: 2026-03-25 15:39:07
   time-done: 2026-03-25 17:54:46

- Created `src/event/command_service.rs` with the `CommandService` trait definition (blocking `serve` method).
- Defined a serializable `CommandError` struct and `CommandErrorKind` enum for structured error replies.
- Re-exported `CommandService`, `CommandError`, and `CommandErrorKind` from `src/event.rs` alongside the other event traits.

## Step - Implement blocking serve on NatsStore
      status: done
time-created: 2026-03-25 15:39:07
   time-done: 2026-03-25 17:56:52

- Created `src/nats/command_service.rs` and implemented `CommandService::serve` for `NatsStore`.
- The endpoint subject is derived from `A::Event::name()`, using the wildcard pattern `<event_name>.*` to capture the aggregate UUID from the last subject token.
- On each request: parse the UUID from the subject, call `read` (`ReplayOneExt`) to load `Root<A>` from sequence 0, deserialize the command from the request payload, call `try_write` (`PublishExt`), reply with an empty body on success or a JSON-serialized `CommandError` with a NATS error status on failure.
- Added `status_code()` method to `CommandError` for mapping error kinds to HTTP-like status codes used by the NATS service error API.
- The `serve` method clones `NatsStore` for each request to obtain a mutable handle for `try_write`, since `NatsStore` is `Clone`.
- Service and group names are derived from `A::Event::name()`, endpoint is named `"command"`.
