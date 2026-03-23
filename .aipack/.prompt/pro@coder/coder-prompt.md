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
  # - derive/**/*.*
  # - compilation_errors.txt

context_globs_post: # Appended after auto-context selection
  # - .aipack/.prompt/pro@coder/dev/plan/*.md

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - src/**/*
  - crates/**/*

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

Improve the structure of the nats implementation of esrc-cqrs, currently all the opcional items like memory Query, cqrs Client, Command aggregate handler are in the root of the nats module.
Improve by creating sub modules and putting these adicional components inside them.
Update the imports in the example to account for that, these adicional components should not be exported in the root.

====
>   Info: Duration: 1m 58s | ~$0.1898 | Prompt Tokens: 18,998 (cached: 0, cache_creation: 0) | Completion Tokens: 8,856 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 12 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Here is the plan:

1. Create sub-modules inside `crates/esrc-cqrs/src/nats/`:
   - `command/` - holds `aggregate_command_handler.rs`
   - `query/` - holds `live_view_query.rs`, `memory_view_query.rs`
   - `client/` - holds `client.rs`

2. Keep `command_dispatcher`, `query_dispatcher`, `projector_runner`, `durable_projector_handler` at the root of `nats/` since they are the core dispatch infrastructure.

3. Update `mod.rs` to reflect the new sub-module structure and only re-export the dispatcher/runner types at the root level.

4. Update the cafe example imports.



Now I need to fix the `command_dispatcher.rs` patch - it has a duplicate import line. Let me reconsider: the original file has `use super::aggregate_command_handler::CommandReply;` and I need to replace it with the new path.



<suggested_git_command>
git add -A .
git commit -a -m "^ esrc-cqrs/nats - Reorganize nats module into command, query, and client sub-modules"
</suggested_git_command>
