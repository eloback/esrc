====
>   Info: Duration: 43s | ~$0.1909 | Prompt Tokens: 20,225 (cached: 0, cache_creation: 0) | Completion Tokens: 3,592 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 9 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the plan, the active step is "NATS KV-backed QueryHandler implementation" which is already implemented (see `src/nats/query_kv.rs`). So I need to move it to done and implement the next todo step: "Vertical slice helper composing ConsumerSpec and QuerySpec".

Let me analyze what's needed:
- A convenience builder that combines a `ReadModel` consumer (`ConsumerSpec`) and a `QueryHandler` (`QuerySpec`) into a single vertical slice declaration.
- A `spawn_read_model_slice` method on `NatsStore` to spawn both the consumer and query service in one call.



The implementation adds:

1. **`ReadModelSlice<P, H>`** in `src/event_modeling.rs`: A builder struct that composes a `ConsumerSpec` (read model consumer) and a `QuerySpec` (query handler) using a shared `ComponentName`. It provides:
   - `new(name, projector, handler)` constructor with sensible defaults (sequential execution, NATS request-reply transport).
   - Override methods for execution policy and query transport.
   - Accessors for both specs, projector, handler, and a destructuring `into_specs()`.

2. **`spawn_read_model_slice`** on `NatsStore` in `src/nats.rs`: A single method that takes a `ReadModelSlice`, destructures it into its two specs, and spawns both the event consumer and the query service as tracked background tasks.

3. **Re-export** of `ReadModelSlice` from `src/lib.rs` alongside the existing `Automation` re-export.

<suggested_git_command>
git add -A . && git commit -a -m "+ event_modeling - Add ReadModelSlice vertical slice helper composing ConsumerSpec and QuerySpec"
</suggested_git_command>
