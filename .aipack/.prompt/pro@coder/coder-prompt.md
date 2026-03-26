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
  - examples/multi-slice-command-service/**/*
  # - crates/esrc-cqrs/**/*
  - derive/**/*.*
  # - compilation_errors.txt

#context_globs_post: # Appended after auto-context selection
# - .aipack/.prompt/pro@coder/dev/plan/*.md

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - src/**/*
  # - crates/**/*

## Set to false to disable file writing (response below this file's prompt)
write_mode: true

## Optimize context files selection (other properties: code_map_model, helper_globs, ..)
auto_context:
  model: flash # (Use a small or inexpensive model)
  input_concurrency: 16 # (default 8)
  enabled: true # (Default true) Comment or set to true to enable.

dev:
  chat: _workbench/queries # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  plan: _workbench/queries # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: opus
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```
now that we have implement the first part of the feature defined in the devchat.
following the plan rules create a todo plan to implement the rest of features.
don't implement anything yet, only generate the todo plan.

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
