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
  - crates/esrc-cqrs/**/*.*
  # - derive/**/*.*
  # - examples/**/*.*
  # - compilation_errors.txt

context_globs_post: # Appended after auto-context selection

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
in my last prompt i asked you implemented the following:
- Add a `query.rs` module under `crates/esrc-cqrs/src/` defining the `QueryHandler` trait, mirroring `CommandHandler` but typed over a request and a response.
- Add a `nats/aggregate_query_handler.rs` for the NATS-backed generic aggregate query handler (loads a read model / aggregate state and returns it serialized).
- Add a `nats/query_dispatcher.rs` for the NATS service-based query dispatcher (same pattern as command dispatcher).
- Wire everything up in `lib.rs`, `nats/mod.rs`, and `registry.rs`.

but you forgot to declare the unit tests defined in crates/esrc-cqrs/tests/integration_nats.rs, fix that mistake

====
>   Info: Duration: 29s | ~$0.08 | Prompt Tokens: 15,275 (cached: 0, cache_creation: 0) | Completion Tokens: 2,280 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 7 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the integration test file, I can see it imports and uses the query-related types but the test file itself doesn't have any tests for the query functionality. The user is saying that the query-related declarations (types, imports) are missing from the integration test file.

Let me look more carefully. The test file imports:

```rust
use esrc_cqrs::nats::{
    AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler,
    NatsCommandDispatcher,
};
```

It's missing imports for the query types (`AggregateQueryHandler`, `QueryEnvelope`, `QueryReply`, `NatsQueryDispatcher`) and there are no query tests defined.

The user says "you forgot to declare the unit tests defined in `crates/esrc-cqrs/tests/integration_nats.rs`" - meaning the query integration tests need to be added to that file.

I need to add:
1. The missing query imports
2. Query-related test cases (send a query, get a response)

Let me write appropriate query integration tests that follow the same pattern as the command tests, using the `Counter` aggregate already defined there.



<suggested_git_command>
git commit -a -m "+ esrc-cqrs - Add query integration tests to integration_nats.rs"
</suggested_git_command>
