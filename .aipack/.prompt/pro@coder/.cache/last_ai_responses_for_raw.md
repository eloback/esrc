====
>   Info: Duration: 14s | ~$0.064 | Prompt Tokens: 18,135 (cached: 0, cache_creation: 0) | Completion Tokens: 639 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 10 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<FILE_CHANGES>

<FILE_NEW file_path="_workbench/service/plan-1-todo-steps.md">
## Step - Add CommandError response type and command_service module skeleton
      status: not_started
time-created: 2026-03-25 15:39:07

- Create `src/event/command_service.rs` with the `CommandService` trait definition (blocking `serve` method).
- Define a serializable `CommandError` struct/enum in that module for structured error replies.
- Re-export `CommandService` from `src/event.rs` alongside the other event traits.
- No implementation yet, only trait definitions and the error type.

## Step - Implement blocking serve on NatsStore
      status: not_started
time-created: 2026-03-25 15:39:07

- Create `src/nats/command_service.rs` and implement `CommandService::serve` for `NatsStore`.
- The endpoint subject is derived from `A::Event::name()`, using the wildcard pattern `<event_name>.*` to capture the aggregate UUID from the last subject token.
- On each request: parse the UUID from the subject, call `read` (`ReplayOneExt`) to load `Root<A>` from sequence 0, deserialize the command from the request payload, call `try_write` (`PublishExt`), reply with an empty body on success or a JSON-serialized `CommandError` with a NATS error status on failure.
- `Aggregate::Command` must implement `serde::de::DeserializeOwned`; enforce this bound at the call site only.
- Wire the new module into `src/nats.rs`.

## Step - Add spawn_service background task variant on NatsStore
      status: not_started
time-created: 2026-03-25 15:39:07

- Define a `CommandServiceExt` trait (or inherent method) with a `spawn_service` method that wraps the blocking `serve` in a background task.
- Integrate with the existing `GracefulShutdown` / `TaskTracker` on `NatsStore` so the spawned task participates in graceful shutdown.
- Re-export or expose `spawn_service` alongside the other NatsStore APIs.
- References: see the definition in plan-3-done-steps.md or plan-2-active-step.md, step "Step - Implement blocking serve on NatsStore".
</FILE_NEW>

</FILE_CHANGES>

<suggested_git_command>
git add -A .
git commit -a -m "+ command-service plan - Add todo steps for CommandService trait and NatsStore implementation"
</suggested_git_command>

