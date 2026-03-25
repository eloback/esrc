====
>   Info: Duration: 27s | ~$0.159 | Prompt Tokens: 21,632 (cached: 0, cache_creation: 0) | Completion Tokens: 2,035 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 14 | Working Files: 0 | Knowledge Files: 2
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the plan, the current active step "Implement blocking serve on NatsStore" is already implemented. The next step in `plan-1-todo-steps.md` is "Add spawn_service background task variant on NatsStore".

Let me implement this step: move the active step to done, activate the next step, and implement it.

I need to add a `spawn_service` method that wraps the blocking `serve` in a background task, integrated with the existing `GracefulShutdown` / `TaskTracker` on `NatsStore`.

Looking at how `GracefulShutdown` works in `src/nats.rs`, I can see:
- `task_tracker: TaskTracker` for tracking spawned tasks
- `exit_tx: Sender<Trigger>` / `exit_rx` for shutdown triggers using `stream_cancel`
- `wait_graceful_shutdown` drops triggers to cancel and then waits

I'll add `spawn_service` as an inherent method on `NatsStore` (not a trait, since it's NATS-specific) that spawns `serve` in a tracked task with cancellation support.



<suggested_git_command>
git commit -a -m "+ nats/command_service - Add spawn_service background task with graceful shutdown"
</suggested_git_command>
