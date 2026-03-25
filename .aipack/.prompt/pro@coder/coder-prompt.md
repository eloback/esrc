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

context_globs_post: # Appended after auto-context selection
  - .aipack/.prompt/pro@coder/dev/plan/*.md

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
  chat: false # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  plan: false # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: claude-sonnet-4-6
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

Following the plan rules create a plan to: Create a CommandService trait that will serve commands to a aggregate.
The Nats implementation will use the Nats service api, the NatsStore will expose a method that returns a async_nats::Service.
The trait implementation will them create a service for that aggregate, and create a listener in a endpoint to receive the commands and invoke the aggregate.
```rs
# proposed interface
let handle = store.aggregate_service::<MyAggregate>(SERVICE_NAME,SERVICE_VERSION).await;
```

====
>   Info: Duration: 25s | ~$0.0598 | Prompt Tokens: 14,346 (cached: 0, cache_creation: 0) | Completion Tokens: 1,120 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 8 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the codebase, I'll create a plan for implementing the `CommandService` trait with NATS service API integration.



<suggested_git_command>
git add -A .
git commit -a -m "+ service - Add plan for CommandService trait with NATS service API"
</suggested_git_command>
