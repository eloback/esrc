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
  - src/**/*.*
  - crates/esrc-cqrs/**/*.*
  - examples/**/*.*
  # - derive/**/*.*
  # - compilation_errors.txt

context_globs_post:        # Appended after auto-context selection
  - .aipack/.prompt/pro@coder/dev/plan/*.md  

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - src/**/*.*
  - crates/**/*.*

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
Following the plan rules, create a plan to do the following:
1. Remove `AggregateQueryHandler` from `esrc-cqrs` (the "load aggregate and project" query pattern).
2. Introduce a `View` trait (like `Aggregate` but without `Command`/`process`/`Error`) in the `esrc` crate.
3. Introduce `LiveViewQuery`: a `QueryHandler` that replays events on each request to build a view.
4. Introduce `MemoryViewQuery`: a `Project` implementation that keeps a view in memory, and a `QueryHandler` that reads from that memory.
5. Update tests and cafe example accordingly.

====
>   Info: Duration: 40s | ~$0.142 | Prompt Tokens: 36,279 (cached: 0, cache_creation: 0) | Completion Tokens: 2,208 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 25 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll analyze the codebase and create a plan for the requested changes.



<suggested_git_commit>
git add -A .
git commit -a -m "+ plan - Add plan for View trait, LiveViewQuery, MemoryViewQuery, and AggregateQueryHandler removal"
</suggested_git_commit>
