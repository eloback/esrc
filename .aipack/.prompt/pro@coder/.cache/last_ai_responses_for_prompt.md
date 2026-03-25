====
>   Info: Duration: 36s | ~$0.1032 | Prompt Tokens: 20,854 (cached: 0, cache_creation: 0) | Completion Tokens: 2,708 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 14 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll implement the next step: "Add spawn_service background task variant on NatsStore".

This involves:
1. Defining a `CommandServiceExt` trait with a `spawn_service` method
2. Implementing it on `NatsStore` with `GracefulShutdown` / `TaskTracker` integration
3. Wiring it into `src/nats.rs` and `src/event.rs`
4. Moving the active step to done and activating the new step



There is one issue I need to flag: the `FILE_PATCH` for `src/event/command_service.rs` adds a blank line after the `use crate::error;` import, which I should avoid doing as a separate hunk. Let me also note that the `plan-1-todo-steps.md` patch removes the only step, leaving the file effectively empty, which is correct (no more todo steps).

A note on the implementation: `stream_cancel` is already a dependency (used in `src/nats.rs` via `stream_cancel::Trigger`), so the `Tripwire` API is available. The `spawn_service` method clones `NatsStore` (which is `Clone`) and spawns the blocking `serve` loop in a tracked task that responds to graceful shutdown.

<suggested_git_command>
git add -A .
git commit -a -m "+ nats/command_service - Add CommandServiceExt trait and spawn_service background task variant"
</suggested_git_command>
