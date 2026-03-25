## Step - Add CommandError response type and command_service module skeleton
      status: done
time-created: 2026-03-25 15:39:07
   time-done: 2026-03-25 15:43:37

- Created `src/event/command_service.rs` with the `CommandService` trait definition (blocking `serve` method).
- Defined a serializable `CommandError` struct and `CommandErrorKind` enum in that module for structured error replies.
- Re-exported `CommandService`, `CommandError`, and `CommandErrorKind` from `src/event.rs` alongside the other event traits.
- No backend implementation in this step; only trait definitions and the error type.

## Step - Implement blocking serve on NatsStore
      status: done
time-created: 2026-03-25 15:39:07
   time-done: 2026-03-25 16:12:06

- Created `src/nats/command_service.rs` and implemented `CommandService::serve` for `NatsStore`.
- The endpoint subject is derived from `A::Event::name()`, using the wildcard pattern `<event_name>.*` to capture the aggregate UUID from the last subject token.
- On each request: parse the UUID from the subject, call `read` (`ReplayOneExt`) to load `Root<A>` from sequence 0, deserialize the command from the request payload, call `try_write` (`PublishExt`), reply with an empty body on success or a JSON-serialized `CommandError` with a NATS error status on failure.
- `Aggregate::Command` must implement `serde::de::DeserializeOwned`; enforce this bound at the call site only.
- Wired the new module into `src/nats.rs`.
- Used `async_nats::service::ServiceExt` to build the NATS service from the existing client handle.
- The `serialize_error` helper provides a safe fallback if the error struct itself cannot be serialized.
- The `#[cfg(feature = "nats")]` guard on the new module mirrors the convention used for other nats sub-modules.

## Step - Add spawn_service background task variant on NatsStore
      status: done
time-created: 2026-03-25 15:39:07
   time-done: 2026-03-25 16:56:09

- Defined `CommandServiceExt` trait in `src/event/command_service.rs` alongside `CommandService`, with a `spawn_service` async method.
- Re-exported `CommandServiceExt` from `src/event.rs`.
- Implemented `CommandServiceExt::spawn_service` on `NatsStore` in `src/nats/command_service.rs`.
- The implementation clones the store (which is `Clone`) and spawns a tokio task via the existing `TaskTracker`.
- `stream_cancel::Tripwire` produces a `(Trigger, Valve)` pair; the `Trigger` is sent to the `GracefulShutdown` exit channel so `wait_graceful_shutdown()` cancels the task by dropping the trigger, firing the valve.
- The spawned task uses `tokio::select!` to race `serve` against the valve, ensuring clean shutdown on signal.
- `A: 'static` bound is required because the spawned task must be `'static`.
