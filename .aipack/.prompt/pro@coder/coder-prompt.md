```yaml
#!meta (parametric prompt)

# See PATH_TO_PRO_CODER_DIR/README.md for additional documentation
# Note: All relative paths are relative to the workspace directory (parent folder of .aipack/)

## Static knowledge (relative & absolute)
knowledge_globs:
  # - /abs/or/rel/path/to/**/*.md      # Any relative or absolute path/glob for Markdown files
  - nats_service.md
  # - pro@coder/README.md              # Pack path supported (here ask question about pro@coder)
  # - core@doc/for-llm/**/*.md         # For help building .aip AIPack agents
  # - pro@rust10x/guide/base/**/*.md   # Rust best practices (require `aip install pro@rust10x`)

## Files the AI will work on (paths & content included in prompt, relative only)
context_globs:
  # - package.json  # for Node.js
  - Cargo.toml    # for Rust
  # - README.md 
  - src/**/*.*      
  - crates/esrc-cqrs/**/*.*
context_globs_post:        # Appended after auto-context selection
  - .aipack/.prompt/pro@coder/dev/plan/*.md  

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - derive/**/*.*
  - src/**/*.*
  - example/cafe/**/*.*
  
## Set to false to disable file writing (response below this file's prompt)
write_mode: true

## Optimize context files selection (other properties: code_map_model, helper_globs, ..)
auto_context: 
  model: flash           # (Use a small or inexpensive model)
  input_concurrency: 16  # (default 8)
  enabled: false         # (Default true) Comment or set to true to enable.

dev:
  chat: false   # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md 
  plan: false   # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: claude-sonnet-4-6

## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```
Following the plan rules, create a plan to do the following: Create a example like the "cafe" that uses the new "esrc-cqrs" crate to run projections and commands.

====
>   Info: Duration: 22s | ~$0.1202 | Prompt Tokens: 34,343 (cached: 0, cache_creation: 0) | Completion Tokens: 1,142 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 38 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the current state of the plan files, I need to create a `plan-1-todo-steps.md` with the steps to build a CQRS example using `esrc-cqrs`. The active step is about fixing cargo check errors, so I'll plan the example steps to come after that work is done.



<suggested_git_command>
git add -A .
git commit -a -m "+ plan - Add cafe CQRS example plan steps"
</suggested_git_command>
