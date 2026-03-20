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
  - src/**/*.*
  # - examples/**/*.*
  # - compilation_errors.txt

context_globs_post: # Appended after auto-context selection

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:

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
implement a missing component in the "esrc-cqrs" a query handler, it should be a special type of command handler that executes a query handler and returns the query result.
it will normally return a Read Model that will be constructed using a projector, don't restrict to only this usecase, but keep that in mind.

====
>   Info: Duration: 1m 5s | ~$0.1016 | Prompt Tokens: 11,487 (cached: 0, cache_creation: 0) | Completion Tokens: 4,479 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 10 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Here is my plan before writing the code:

- Add a `query.rs` module under `crates/esrc-cqrs/src/` defining the `QueryHandler` trait, mirroring `CommandHandler` but typed over a request and a response.
- Add a `nats/aggregate_query_handler.rs` for the NATS-backed generic aggregate query handler (loads a read model / aggregate state and returns it serialized).
- Add a `nats/query_dispatcher.rs` for the NATS service-based query dispatcher (same pattern as command dispatcher).
- Wire everything up in `lib.rs`, `nats/mod.rs`, and `registry.rs`.

The query handler is intentionally kept general: the request type and response type are both user-defined, so it works for read-model projector results, aggregate state queries, or anything else.



<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs - Add QueryHandler trait, AggregateQueryHandler, NatsQueryDispatcher, and registry integration"
</suggested_git_command>
