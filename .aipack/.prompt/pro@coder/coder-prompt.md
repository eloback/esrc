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
  # - docs/skill/**/*

## Files the AI will work on (paths & content included in prompt, relative only)
context_globs:
  - Cargo.toml # for Rust
  - src/**/*
  - examples/**/*
  # - examples/multi-slice-command-service/**/*
  # - examples/basic-query-service/**/*
  # - crates/esrc-cqrs/**/*
  # - derive/**/*.*
  # - compilation_errors.txt

#context_globs_post: # Appended after auto-context selection
# - .aipack/.prompt/pro@coder/dev/plan/*.md

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - src/**/*
  - examples/**/*
  # - crates/**/*

## Set to false to disable file writing (response below this file's prompt)
write_mode: true

## Optimize context files selection (other properties: code_map_model, helper_globs, ..)
auto_context:
  model: flash # (Use a small or inexpensive model)
  input_concurrency: 16 # (default 8)
  enabled: true # (Default true) Comment or set to true to enable.

dev:
  # chat: _workbench/queries # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  plan: _workbench/integration_between_bounded_contexts # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: opus
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

following the plan rules create a todo plan to update the command_service and it's Nats implementation to have the bounded context in the service name of the Nats implementation.
When a project have multiple bounded contexts the prefix of the NatsStore will be the bounded context, so a application that have many bounded contexts will have multiple NatsStores.
Verify how our framework support this usecase and if necessary create a plan to improve it.
do not update any files, only the plan todo.

====
>   Info: Duration: 23s | ~$0.1246 | Prompt Tokens: 19,917 (cached: 0, cache_creation: 0) | Completion Tokens: 1,000 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 6 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the codebase, I need to analyze how the command service and its NATS implementation handle bounded context naming, and whether multiple `NatsStore` instances (one per bounded context) are properly supported.

**Key observations:**

1. **`NatsStore::try_new`** takes a `prefix` which is the bounded context name (e.g., `BOUNDED_CONTEXT_NAME` in the example).

2. **`CommandService::serve`** in `src/nats/command_service.rs` creates a NATS service with `event_name` as the service name and uses `event_name` as the group name. The endpoint subject is `<event_name>.command.*`. This means if two bounded contexts have aggregates with the same event name, there would be a collision.

3. **`CommandClient::send_command`** uses subject `<event_name>.command.<id>`, which also lacks bounded context scoping.

4. The NATS service name should incorporate the bounded context prefix to avoid collisions between bounded contexts.

5. Multiple `NatsStore` instances per application are already supported structurally (the example creates one per bounded context), but the command service subjects/names need to be scoped by the prefix.



<suggested_git_command>
git add -A . && git commit -m "+ integration_between_bounded_contexts - Create todo plan for scoping NATS command service with bounded context prefix"
</suggested_git_command>
