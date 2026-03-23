====
>   Info: Duration: 48s | ~$0.1996 | Prompt Tokens: 48,132 (cached: 0, cache_creation: 0) | Completion Tokens: 3,678 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 46 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll implement the first step: **Define NatsServiceCommandHandler trait and adapter**.

Let me analyze what needs to be done:

1. Add `NatsServiceCommandHandler<S, C>` trait to `crates/esrc-cqrs/src/command.rs`
2. Create `crates/esrc-cqrs/src/nats/command/service_command_handler.rs`
3. Update `crates/esrc-cqrs/src/nats/command/mod.rs`
4. Update `crates/esrc-cqrs/src/nats/mod.rs`
5. Update `crates/esrc-cqrs/src/lib.rs`
6. Update plan files



<suggested_git_command>
git add -A .
git commit -m "+ esrc-cqrs - Add NatsServiceCommandHandler trait and ServiceCommandHandler adapter"
</suggested_git_command>
