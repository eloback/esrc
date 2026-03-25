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
  - nats_service.md

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
model: opus
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

Following the plan rules, implement the next step in the plan.

====
>   Info: Duration: 27s | ~$0.159 | Prompt Tokens: 21,632 (cached: 0, cache_creation: 0) | Completion Tokens: 2,035 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 14 | Working Files: 0 | Knowledge Files: 2
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the plan, the current active step "Implement blocking serve on NatsStore" is already implemented. The next step in `plan-1-todo-steps.md` is "Add spawn_service background task variant on NatsStore".

Let me implement this step: move the active step to done, activate the next step, and implement it.

I need to add a `spawn_service` method that wraps the blocking `serve` in a background task, integrated with the existing `GracefulShutdown` / `TaskTracker` on `NatsStore`.

Looking at how `GracefulShutdown` works in `src/nats.rs`, I can see:
- `task_tracker: TaskTracker` for tracking spawned tasks
- `exit_tx: Sender<Trigger>` / `exit_rx` for shutdown triggers using `stream_cancel`
- `wait_graceful_shutdown` drops triggers to cancel and then waits

I'll add `spawn_service` as an inherent method on `NatsStore` (not a trait, since it's NATS-specific) that spawns `serve` in a tracked task with cancellation support.



<suggested_git_command>
git commit -a -m "+ nats/command_service - Add spawn_service background task with graceful shutdown"
</suggested_git_command>
