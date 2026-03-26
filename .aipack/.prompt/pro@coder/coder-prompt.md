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
  # - nats_service.md

## Files the AI will work on (paths & content included in prompt, relative only)
context_globs:
  - Cargo.toml # for Rust
  - src/**/*
  # - examples/multi-slice-command-service/**/*
  # - examples/basic-query-service/**/*
  # - crates/esrc-cqrs/**/*
  # - derive/**/*.*
  # - compilation_errors.txt

#context_globs_post: # Appended after auto-context selection
# - .aipack/.prompt/pro@coder/dev/plan/*.md

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - src/**/*
  - examples/**/*
  # - crates/**/*

## Set to false to disable file writing (response below this file's prompt)
write_mode: true

## Optimize context files selection (other properties: code_map_model, helper_globs, ..)
auto_context:
  model: flash # (Use a small or inexpensive model)
  input_concurrency: 16 # (default 8)
  enabled: true # (Default true) Comment or set to true to enable.

dev:
  # chat: _workbench/queries # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  plan: _workbench/queries # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: opus
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

following the plan rules implement the next step in the plan.

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
