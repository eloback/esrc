====
>   Info: Duration: 39s | ~$0.1857 | Prompt Tokens: 27,432 (cached: 0, cache_creation: 0) | Completion Tokens: 1,943 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 11 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the dev-chat design summary, the done steps, and the "What Is NOT Included in the Initial Implementation" section, I can identify the remaining features to plan.

The completed steps cover:
1. ✅ Rename `ConsumerName` to `ComponentName`
2. ✅ Core traits (`Query`, `QueryHandler`, `QueryTransport`, `QuerySpec`) in `src/query.rs`
3. ✅ `QueryService` and `QueryClient` traits in `src/query.rs`
4. ✅ NATS `QueryService`/`QueryClient` implementation

Remaining from the design summary's deferred list:
- In-memory `QueryHandler` helper for `View`-based live projections
- NATS KV-backed `QueryHandler` implementation
- Vertical slice helper/builder that composes a `ConsumerSpec` + `QuerySpec`
- Caching layer (decorator pattern)
- Pagination/sorting helper types



<suggested_git_command>
git commit -a -m ". queries plan - Add todo steps for remaining query framework features"
</suggested_git_command>
