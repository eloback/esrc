## Step - Add CommandError response type and command_service module skeleton
      status: active
time-created: 2026-03-25 15:39:07
time-current: 2026-03-25 17:45:50

- Create `src/event/command_service.rs` with the `CommandService` trait definition (blocking `serve` method).
- Define a serializable `CommandError` struct/enum in that module for structured error replies.
- Re-export `CommandService` from `src/event.rs` alongside the other event traits.
- No implementation yet, only trait definitions and the error type.
