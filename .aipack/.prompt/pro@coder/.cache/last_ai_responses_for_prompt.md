====
>   Info: Duration: 1m 5s | ~$0.1016 | Prompt Tokens: 11,487 (cached: 0, cache_creation: 0) | Completion Tokens: 4,479 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 10 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Here is my plan before writing the code:

- Add a `query.rs` module under `crates/esrc-cqrs/src/` defining the `QueryHandler` trait, mirroring `CommandHandler` but typed over a request and a response.
- Add a `nats/aggregate_query_handler.rs` for the NATS-backed generic aggregate query handler (loads a read model / aggregate state and returns it serialized).
- Add a `nats/query_dispatcher.rs` for the NATS service-based query dispatcher (same pattern as command dispatcher).
- Wire everything up in `lib.rs`, `nats/mod.rs`, and `registry.rs`.

The query handler is intentionally kept general: the request type and response type are both user-defined, so it works for read-model projector results, aggregate state queries, or anything else.



<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs - Add QueryHandler trait, AggregateQueryHandler, NatsQueryDispatcher, and registry integration"
</suggested_git_command>
