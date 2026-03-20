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
  - Cargo.toml    # for Rust
  # - README.md 
  - src/**/*.*      
  - crates/esrc-cqrs/**/*.*
  - examples/**/*.*

context_globs_post:        # Appended after auto-context selection

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - derive/**/*.*
  - compilation_errors.txt
  
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
fix the compilation errors on the cafe example, i was trying to use the esrc-cqrs crate to test if it works, so you can't remove it and call it a day.

====
>   Info: Duration: 17s | ~$0.1248 | Prompt Tokens: 36,165 (cached: 0, cache_creation: 0) | Completion Tokens: 1,084 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 43 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the compilation errors, I need to analyze the cafe example. The `examples/cafe/main.rs` references `mod domain` and `mod projector`, but the file structure shows there's also `examples/cafe/tab.rs`, `examples/cafe/table.rs`, and `examples/cafe/error.rs` which aren't declared. More importantly, the `main.rs` uses `esrc_cqrs` but the `Cargo.toml` example entry doesn't include `esrc-cqrs` as a dependency for the example.

Let me trace the issues:

1. `examples/cafe/main.rs` declares `mod domain` and `mod projector` but NOT `mod tab`, `mod table`, `mod error` - these files exist but aren't used in main, which is fine.
2. The `Cargo.toml` example entry only has `required-features = ["nats", "derive"]` but `esrc-cqrs` needs to be a dev-dependency.
3. Looking at `crates/esrc-cqrs/Cargo.toml`, it exists as a workspace member, so it needs to be added as a dev-dependency in the root `Cargo.toml`.
4. In `examples/cafe/main.rs`, `ConnectOptions` is imported but not used.
5. The `error` module is in `examples/cafe/error.rs` but not declared in `main.rs` - but it's needed by `tab.rs` which references `TabError`. Since `tab.rs` isn't used in `main.rs`, that's fine.
6. The `write` method in `publish.rs` returns `Root::with_aggregate(aggregate, id, last_sequence)` but should use the NEW sequence from the publish result - that's a pre-existing bug, not our concern.
7. `examples/cafe/main.rs` imports `error` module (unused import of `ConnectOptions`).

The main fix needed: add `esrc-cqrs` as a dev-dependency in root `Cargo.toml` and fix the unused import in `main.rs`.



Wait, let me re-examine more carefully. The `required-features` shouldn't need `kurrent`. Let me also check if `ConnectOptions` is actually unused.

In `main.rs`, `ConnectOptions` is imported but `async_nats::connect(NATS_URL).await?` is used directly without it - so it's unused. Also, `esrc-cqrs` needs to be a proper dependency for the example.



<suggested_git_command>
git commit -a -m "^ cafe example - Add esrc-cqrs dev-dependency and fix unused import"
</suggested_git_command>
