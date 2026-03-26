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
following the plan rules, executes the next step in the plan.

====
>   Info: Duration: 27s | ~$0.2017 | Prompt Tokens: 29,995 (cached: 0, cache_creation: 0) | Completion Tokens: 2,069 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 11 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the plan, the first step in `plan-1-todo-steps.md` is "Rename ConsumerName to ComponentName in event_modeling.rs". Let me implement it now.



<suggested_git_command>
git add -A . && git commit -a -m "> event_modeling - Rename ConsumerName to ComponentName and consumer field to component"
</suggested_git_command>
