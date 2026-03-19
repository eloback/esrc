====
>   Info: Duration: 20s | ~$0.1301 | Prompt Tokens: 38,956 (cached: 0, cache_creation: 0) | Completion Tokens: 880 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 36 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Here is the plan for fixing the `cargo check` errors in `esrc-cqrs`.

<FILE_CHANGES>

<FILE_NEW file_path=".aipack/.prompt/pro@coder/dev/plan/plan-1-todo-steps.md">
## Step - Fix cargo check errors and warnings in esrc-cqrs
      status: not_started
time-created: 2026-03-19 19:23:51

Fix all errors and warnings reported by `cargo check`:

- **`crates/esrc-cqrs/Cargo.toml`**: add `uuid` with the `serde` feature so that `Uuid` implements `Serialize`/`Deserialize`.

- **`crates/esrc-cqrs/src/nats/aggregate_command_handler.rs`**: remove the unused imports (`std::pin::pin`, `Error`, `IntoSendFuture`, `Context`, `futures::StreamExt`). Fix the `durable_observe` call: the method lives on the `custom` impl block inside `esrc::nats`, so import/call it correctly, or use the public `durable_observe` path on `NatsStore`.

- **`crates/esrc-cqrs/src/nats/aggregate_projector_handler.rs`**: remove unused imports (`std::pin::pin`, `Error`, `IntoSendFuture`, `Context`, `futures::StreamExt`). Fix the missing `durable_observe` method: `NatsStore::durable_observe` is defined in `src/nats/event.rs` under `pub mod custom` but is not re-exported from `esrc::nats`. Since it is a method on `NatsStore` defined in the `custom` submod (an inherent impl), it should be accessible directly on `&NatsStore` once the module is compiled. Investigate why it is not found and expose it correctly.

- **`crates/esrc-cqrs/src/nats/command_dispatcher.rs`**: remove unused import `esrc::nats::NatsStore`.

- **`crates/esrc-cqrs/src/registry.rs`**: add `+ Sync` bound to `register_projector`'s `H` type parameter to satisfy `ErasedProjectorHandler`.
</FILE_NEW>

</FILE_CHANGES>

Looking at the errors more carefully, I can see this is all actionable in a single plan step. The issues are:

1. `uuid` needs the `serde` feature in `esrc-cqrs/Cargo.toml`.
2. Unused imports in `aggregate_projector_handler.rs` and `command_dispatcher.rs`.
3. `durable_observe` is defined as an inherent method inside `pub mod custom` inside `src/nats/event.rs`, but `custom` is not re-exported from `esrc::nats`, so it is not visible from the crate. The fix is to call it via the path that is actually reachable, or since it is an inherent method on `NatsStore`, it should be visible as long as the module compiles into scope (which it does since `event` is `pub mod event` in `src/nats.rs`). The real issue is that the `run` method receives `&NatsStore` (shared ref) but `durable_observe` takes `&self` too, so this should work once the visibility is resolved by the module structure.
4. `register_projector` needs `H: Sync` bound.

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs/plan - Add plan to fix cargo check errors and warnings"
</suggested_git_command>

