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
>   Info: Duration: 27s | ~$0.1495 | Prompt Tokens: 39,514 (cached: 0, cache_creation: 0) | Completion Tokens: 2,064 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 28 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll analyze the plan files and implement the next step: "Introduce the View trait in esrc".

This step requires:
1. Creating `src/view.rs` with the `View` trait
2. Adding `pub mod view;` to `src/lib.rs`
3. Re-exporting `View` from `src/lib.rs`

Let me also move the active step to done and activate this new step.



<suggested_git_command>
git add -A .
git commit -m "+ esrc::View - Add View trait for read models built from event streams"
</suggested_git_command>

Here is a summary of what was done:

- Created `src/view.rs` with the `View` trait:
  - `type Event: crate::event::Event` as the associated event type
  - `fn apply(self, event: &Self::Event) -> Self` as the only required method
  - Bounded by `Default + Send`, no `Command`, `process`, or `Error`
  - Includes a doc example showing a `CounterView` implementation

- Updated `src/lib.rs`:
  - Added `pub mod view;` with a doc comment
  - Added `pub use view::View;` to re-export `View` at the crate root

- Updated plan files:
  - Moved the previous active step ("Remove AggregateQueryHandler") to `plan-3-done-steps.md`
  - Moved "Introduce the View trait in esrc" from todo to active in `plan-2-active-step.md`
  - Removed the step from `plan-1-todo-steps.md`
