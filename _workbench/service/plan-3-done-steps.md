## Step - Add CommandError response type and command_service module skeleton
      status: done
time-created: 2026-03-25 15:39:07
   time-done: 2026-03-25 15:43:37

- Created `src/event/command_service.rs` with the `CommandService` trait definition (blocking `serve` method).
- Defined a serializable `CommandError` struct and `CommandErrorKind` enum in that module for structured error replies.
- Re-exported `CommandService`, `CommandError`, and `CommandErrorKind` from `src/event.rs` alongside the other event traits.
- No backend implementation in this step; only trait definitions and the error type.
