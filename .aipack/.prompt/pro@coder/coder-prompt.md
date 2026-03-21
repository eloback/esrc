```yaml
#!meta (parametric prompt)

# See PATH_TO_PRO_CODER_DIR/README.md for additional documentation
# Note: All relative paths are relative to the workspace directory (parent folder of .aipack/)

## Static knowledge (relative & absolute)
knowledge_globs:
  # - /abs/or/rel/path/to/**/*.md      # Any relative or absolute path/glob for Markdown files
  # - pro@coder/README.md              # Pack path supported (here ask question about pro@coder)
  # - core@doc/for-llm/**/*.md         # For help building .aip AIPack agents
  # - pro@rust10x/guide/base/**/*.md   # Rust best practices (require `aip install pro@rust10x`)

## Files the AI will work on (paths & content included in prompt, relative only)
context_globs:
  # - package.json  # for Node.js
  - Cargo.toml # for Rust
  # - README.md
  - src/**/*
  - crates/esrc-cqrs/**/*
  - examples/cafe/**/*
  # - derive/**/*.*
  # - compilation_errors.txt

context_globs_post:        # Appended after auto-context selection
  - .aipack/.prompt/pro@coder/dev/plan/*.md  

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - src/**/*
  - crates/**/*
  - examples/**/*

## Set to false to disable file writing (response below this file's prompt)
write_mode: true

## Optimize context files selection (other properties: code_map_model, helper_globs, ..)
auto_context:
  model: gpt-5.4-mini # (Use a small or inexpensive model)
  input_concurrency: 8 # (default 8)
  enabled: true # (Default true) Comment or set to true to enable.

dev:
  chat: false # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  plan: false # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: claude-sonnet-4-6
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```
Following the plan rules, execute the next step in the plan and update the appropriate files.

====
>   Info: Duration: 1m 4s | ~$0.2271 | Prompt Tokens: 48,866 (cached: 0, cache_creation: 0) | Completion Tokens: 5,367 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 40 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll execute the next step from the plan: "Introduce the View trait in esrc". Looking at the active step, `src/view.rs` already exists (shown in the context files), and `src/lib.rs` already has `pub mod view;` and `pub use view::View;`. Let me check what's actually needed.

Looking at the existing `src/view.rs` content provided - it already has a full `View` trait implementation! And `src/lib.rs` already exports it. The active step appears to already be implemented. I need to move the active step to done and implement the next todo step: "Introduce LiveViewQuery in esrc-cqrs".

Let me implement `LiveViewQuery`:

- Create `crates/esrc-cqrs/src/nats/live_view_query.rs`
- Export from `mod.rs`
- Move active step to done, activate next step



Now I need to check what `ReplayOneExt` provides. Looking at `src/event/replay.rs` - I see `ReplayOne` and `ReplayOneExt` traits referenced. The `ReplayOneExt` likely has a `read` method that returns deserialized events. However, I need to verify the exact API. Looking at the existing integration test, it uses `store.read(aggregate_id)` which returns a `Root<Counter>`. That's the aggregate read path. For views, I need the raw event stream.

The `replay_one` method returns a stream of `NatsEnvelope`, not deserialized events. I should use the envelope stream and deserialize manually. Let me correct the implementation:



Wait, `deserialize` on `NatsEnvelope` requires `DeserializeVersion + Event`. Also I need to import `Envelope` trait. Let me look again at the full implementation and provide a complete corrected file:



<suggested_git_command>
git add -A .
git commit -m "+ esrc-cqrs - Add LiveViewQuery handler that replays View events on each query request"
</suggested_git_command>

Here is a summary of what was done:

**Moved to done:** "Introduce the View trait in esrc" - already implemented in `src/view.rs` and exported from `src/lib.rs`.

**Activated and implemented:** "Introduce LiveViewQuery in esrc-cqrs"

- Created `crates/esrc-cqrs/src/nats/live_view_query.rs` defining `LiveViewQuery<V, R>`:
  - Generic over `V: View` (the view to build) and `R: Serialize` (the projected response type).
  - Constructor `new(handler_name, projection)` accepting a routing name and a `fn(&V) -> R` projection function.
  - `QueryHandler<NatsStore>` impl that:
    1. Deserializes payload as `QueryEnvelope` (from `query_dispatcher`).
    2. Calls `store.replay_one::<V::Event>(id, Default::default())` to get the full event stream.
    3. Folds each replayed envelope (deserialized via `Envelope::deserialize`) into `V::default()` using `View::apply`.
    4. Applies the projection function and serializes the result as a `QueryReply`.
- Exported `LiveViewQuery` from `crates/esrc-cqrs/src/nats/mod.rs`.
