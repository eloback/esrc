====
>   Info: Duration: 1m 58s | ~$0.1898 | Prompt Tokens: 18,998 (cached: 0, cache_creation: 0) | Completion Tokens: 8,856 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 12 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Here is the plan:

1. Create sub-modules inside `crates/esrc-cqrs/src/nats/`:
   - `command/` - holds `aggregate_command_handler.rs`
   - `query/` - holds `live_view_query.rs`, `memory_view_query.rs`
   - `client/` - holds `client.rs`

2. Keep `command_dispatcher`, `query_dispatcher`, `projector_runner`, `durable_projector_handler` at the root of `nats/` since they are the core dispatch infrastructure.

3. Update `mod.rs` to reflect the new sub-module structure and only re-export the dispatcher/runner types at the root level.

4. Update the cafe example imports.



Now I need to fix the `command_dispatcher.rs` patch - it has a duplicate import line. Let me reconsider: the original file has `use super::aggregate_command_handler::CommandReply;` and I need to replace it with the new path.



<suggested_git_command>
git add -A .
git commit -a -m "^ esrc-cqrs/nats - Reorganize nats module into command, query, and client sub-modules"
</suggested_git_command>
