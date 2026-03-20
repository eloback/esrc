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
  - examples/**/*.*
  # - derive/**/*.*
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
Update the cafe example to feature queries the new feature of the esrc-cqrs crate.
Update also the query dispatch logic in the example, we now that the erros are wrapped in our new esrc_cqrs::Error type.

====
>   Info: Duration: 38s | ~$0.0878 | Prompt Tokens: 14,864 (cached: 0, cache_creation: 0) | Completion Tokens: 2,881 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the existing code, I need to:

1. Update `examples/cafe/domain.rs` to add a query response type (`OrderState`).
2. Update `examples/cafe/projector.rs` - no changes needed there.
3. Add a new `examples/cafe/query.rs` with an `OrderQueryHandler` (or use `AggregateQueryHandler`).
4. Update `examples/cafe/main.rs` to register a query handler and demonstrate sending a query, using the `NatsQueryDispatcher` and wrapping errors with `esrc_cqrs::Error`.

I also need to check that the `NatsQueryDispatcher` is used properly and that errors are wrapped in `esrc_cqrs::Error` as noted.

Looking at `main.rs`, it currently only shows commands. I need to:
- Add a query to fetch order state after placing it.
- Register a `AggregateQueryHandler` for `Order`.
- Run a `NatsQueryDispatcher` alongside the command dispatcher.
- Show that query errors come back as `esrc_cqrs::Error`.

I also need to add an `OrderState` read model to `domain.rs` and derive the right traits.

Let me also check what needs to be added to `Cargo.toml` for the example - it already has `esrc-cqrs` as a dev-dependency, so that's fine.



<suggested_git_command>
git add -A .
git commit -a -m "^ cafe example - Add query handler registration and query dispatch with esrc_cqrs::Error"
</suggested_git_command>
