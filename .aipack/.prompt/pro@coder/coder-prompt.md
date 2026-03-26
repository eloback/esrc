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
  - docs/skill/**/*

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
  # plan: _workbench/queries # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: opus
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

aggregate all examples in one big modular example, it should test:

- a Command Service
- one Automation
- One of each type of default Read Model
- The Command Client
- The Query Client

some notes on the implementationd.

- use the other examples as reference.
- create a domain folder where you will define the events commands and aggregates in different files
- separate the code components in slices ex: ("create_operation", "send_email", "memory_operation_list")
- The Automation should emit a event that will update the read models (projected ones at least)
- The Read model will be queried to validate the flow working properly
- do not clear the storage on start, put some assertions on the end to validate that the read models are being properly updated.

====
>   Info: Duration: 1m 22s | ~$0.3535 | Prompt Tokens: 34,658 (cached: 0, cache_creation: 0) | Completion Tokens: 7,207 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 15 | Working Files: 0 | Knowledge Files: 4
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the existing examples and the codebase, I'll create a comprehensive modular example that tests all the required components.



Here is a summary of the structure:

- **`domain/`** - Contains aggregates, events, and commands split into `operation.rs` and `email.rs`, with `mod.rs` exposing `BOUNDED_CONTEXT_NAME`.
- **`create_operation/`** - Automation slice that reacts to `OperationCreated` events and sends a `SendNotification` command to the `EmailAggregate`.
- **`memory_operation_list/`** - In-memory read model slice using `InMemoryViewStore` + `ReadModelSlice` to materialize and query operations.
- **`kv_operation_view/`** - NATS KV read model slice using `NatsKvStore` + `ReadModelSlice` to persist operation views in a JetStream KV bucket.
- **`send_email/`** - In-memory read model slice that materializes `EmailEvent` records, proving the automation triggered the email command successfully.
- **`main.rs`** - Orchestrates everything: spawns command services, automation, both read model types, sends commands via `CommandClient`, then queries via `QueryClient` with assertions to validate the entire flow.

<suggested_git_command>
git add -A . && git commit -m "+ full-slice-example - Add comprehensive example testing command service, automation, in-memory and KV read models, command client, and query client"
</suggested_git_command>
