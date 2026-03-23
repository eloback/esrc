====
>   Info: Duration: 45s | ~$0.1261 | Prompt Tokens: 25,592 (cached: 0, cache_creation: 0) | Completion Tokens: 3,288 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll analyze the current boilerplate and design a plan to improve the developer experience for both commands and queries in the `esrc-cqrs` framework.

## Analysis of Current Boilerplate

For a command, the caller must:
1. Construct a `CommandEnvelope` manually
2. Serialize it with `serde_json::to_vec`
3. Build the subject string via `command_dispatcher::command_subject`
4. Call `nats_client.request(...)`
5. Deserialize the `CommandReply`
6. Check `reply.success` and branch

For a query, the same steps apply with `QueryEnvelope` and `QueryReply`.

## Plan

### Phase 1: `NatsClient` wrapper (no proc macro needed yet)

Create a high-level `EsrcNatsClient` (or `CqrsClient`) struct that wraps an `async_nats::Client` and exposes ergonomic `send_command` and `send_query` methods. This eliminates all the boilerplate in a single call with typed generics, no derive macros needed.

```rust
// Target DX
let reply = client.send_command("clt-service", "Escrituracao", batch_id, cmd).await?;
let state: CounterState = client.send_query("clt-service", "Counter.GetState", id).await?;
```

### Phase 2: `esrc_cqrs_macros` proc macro crate (optional enhancement)

A `#[cqrs_command]` and `#[cqrs_query]` attribute macro on handler structs that auto-derives the service/handler name constants and generates a typed `dispatch` associated function, so callers never construct envelopes manually.

### Immediate deliverable: Phase 1

This gives the biggest DX win with zero proc macros and zero new dependencies. I'll implement:

1. `crates/esrc-cqrs/src/nats/client.rs` - the `CqrsClient` struct
2. Export it from `mod.rs`
3. Add a usage example in the cafe example

Let me implement Phase 1 now:



Now here is the improved DX the plan delivers, shown as a before/after comparison:

**Before (current boilerplate):**
```rust
let command_envelope = CommandEnvelope { id: body.batch_id, command: cmd };
let payload = serde_json::to_vec(&command_envelope).expect("serialize");
let subject = esrc_cqrs::nats::command_dispatcher::command_subject("clt-service", "Escrituracao");
let reply = state.nats_client.request(subject, payload.into()).await.unwrap();
let reply: CommandReply = serde_json::from_slice(&reply.payload).unwrap();
if !reply.success { return Err(ErrorResponses::from(reply.error.unwrap())); }
```

**After (with `CqrsClient`):**
```rust
// Option A: get typed result directly, failure becomes Err
let id = state.cqrs.dispatch_command("clt-service", "Escrituracao", body.batch_id, cmd).await?;

// Option B: get the raw reply if you need to inspect the error type
let reply = state.cqrs.send_command("clt-service", "Escrituracao", body.batch_id, cmd).await?;

// Query - typed deserialization in one call
let state: MyState = cqrs.dispatch_query("clt-service", "MyAggregate.GetState", id).await?;
```

**Two-method design per verb:**

- `send_command` / `send_query`: returns the raw reply envelope (gives access to the `CommandReply::error` typed field for downcasting aggregate errors).
- `dispatch_command` / `dispatch_query`: converts `success=false` into `Err` automatically, for the common case where you just want to propagate failures.

**Phase 2 (proc macro) would add**, when warranted by usage patterns: a `#[cqrs_handler(service = "...", name = "...")]` attribute that generates an associated `dispatch` method directly on the command type, so the service/handler name strings are co-located with the type definition rather than scattered across call sites.

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs/nats - Add CqrsClient for ergonomic command and query dispatch"
</suggested_git_command>
