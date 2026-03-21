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
>   Info: Duration: 17s | ~$0.1746 | Prompt Tokens: 52,235 (cached: 0, cache_creation: 0) | Completion Tokens: 1,192 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 45 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

The active step is the last one in the plan. I'll finalize it (move to done) and inform the user there are no more steps.

Looking at the active step, it covers updating tests and the cafe example. The implementation was completed (including the bug fix for `LiveViewQuery`). I just need to move it to done and update the plan files accordingly.



All steps in the plan are now complete. The `plan-2-active-step.md` has been cleared and the final step has been archived to `plan-3-done-steps.md` with `status: done`.

There are **no more steps** in `plan-1-todo-steps.md`. All planned work has been implemented and archived.

<suggested_git_command>
git commit -a -m ". plan - Finalize active step and archive to done; no remaining todo steps"
</suggested_git_command>
