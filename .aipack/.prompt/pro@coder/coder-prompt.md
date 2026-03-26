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
create a basic example based on multi-slice, that validate the working of the query handler and query client.

====
>   Info: Duration: 1m 1s | ~$0.2713 | Prompt Tokens: 30,444 (cached: 0, cache_creation: 0) | Completion Tokens: 4,764 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the existing `multi-slice-command-service` example, I need to create a similar example that demonstrates the `QueryHandler` and `QueryClient` working with the NATS backend. Let me study the existing patterns and create a basic example.

I'll create an example that:
1. Defines a simple domain with events and an aggregate
2. Creates a `Project` implementation that maintains a read model
3. Implements a `QueryHandler` for that read model
4. Uses `QueryService` (via `spawn_query_service`) and `QueryClient` (via `get_by_id` and `query`) to validate the round-trip



Wait, I need to also check that the plan-1-todo-steps doesn't have a duplicate. Let me re-read my patch. The patch adds the active step at the top of todo, but per the rules, the step should be moved from todo to active (not remain in todo). Let me fix the plan files properly.



<suggested_git_command>
git add -A . && git commit -a -m "+ basic-query-service - Add example validating QueryHandler and QueryClient over NATS"
</suggested_git_command>
