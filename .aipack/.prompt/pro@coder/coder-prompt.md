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
  - Cargo.toml # for Rust
  - src/**/*
  # - crates/esrc-cqrs/**/*
  # - derive/**/*.*
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
  chat: _workbench/service # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  plan: _workbench/service # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: claude-sonnet-4-6
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

Following the plan rules, implement the next step in the plan.

====
>   Info: Duration: 36s | ~$0.1032 | Prompt Tokens: 20,854 (cached: 0, cache_creation: 0) | Completion Tokens: 2,708 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 14 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll implement the next step: "Add spawn_service background task variant on NatsStore".

This involves:
1. Defining a `CommandServiceExt` trait with a `spawn_service` method
2. Implementing it on `NatsStore` with `GracefulShutdown` / `TaskTracker` integration
3. Wiring it into `src/nats.rs` and `src/event.rs`
4. Moving the active step to done and activating the new step



There is one issue I need to flag: the `FILE_PATCH` for `src/event/command_service.rs` adds a blank line after the `use crate::error;` import, which I should avoid doing as a separate hunk. Let me also note that the `plan-1-todo-steps.md` patch removes the only step, leaving the file effectively empty, which is correct (no more todo steps).

A note on the implementation: `stream_cancel` is already a dependency (used in `src/nats.rs` via `stream_cancel::Trigger`), so the `Tripwire` API is available. The `spawn_service` method clones `NatsStore` (which is `Clone`) and spawns the blocking `serve` loop in a tracked task that responds to graceful shutdown.

<suggested_git_command>
git add -A .
git commit -a -m "+ nats/command_service - Add CommandServiceExt trait and spawn_service background task variant"
</suggested_git_command>
