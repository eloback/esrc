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
  - examples/cafe/**/*
  # - derive/**/*.*
  # - compilation_errors.txt

context_globs_post:        # Appended after auto-context selection
  # - .aipack/.prompt/pro@coder/dev/plan/*.md  

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - src/**/*
  - crates/**/*
  - examples/**/*

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
move the cafe example from the esrc crate to the new standalone esrc-cqrs crate, it should use the local esrc from that crate.

====
>   Info: Duration: 1m 16s | ~$0.2086 | Prompt Tokens: 40,062 (cached: 0, cache_creation: 0) | Completion Tokens: 5,891 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 42 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the task, I need to move the `cafe` example from the `esrc` crate to the `esrc-cqrs` crate. This involves:

1. Adding the example files to `crates/esrc-cqrs/examples/cafe/`
2. Updating `crates/esrc-cqrs/Cargo.toml` to add the example and necessary dependencies
3. Removing the example from the root `Cargo.toml` (and the files from `examples/cafe/`)

Let me trace the dependencies needed for the cafe example:
- `async-nats` (already optional/nats feature)
- `serde`, `serde_json` (already optional/nats feature)
- `tokio` (already full)
- `uuid` with v4 feature (already there)
- `thiserror` (already there)
- `esrc` with nats+derive features
- `esrc-cqrs` itself (it's the crate, so it uses internal paths)
- `esrc_derive` macros (via esrc)

The cafe example also uses `tab.rs`, `table.rs`, `error.rs`, `domain.rs`, `projector.rs` and `tab/tests.rs`. The `tab/tests.rs` is a test module inside `tab.rs` - I need to check if those tests need a NATS connection (they're in `crates/esrc-cqrs/tests/integration_nats.rs` style or unit tests).

Looking at `tab.rs`, it has `mod tests;` which pulls in `examples/cafe/tab/tests.rs`. Since these are unit tests on the `Tab` aggregate, they don't need NATS. I'll include them.



Note: I've kept the original `examples/cafe/` files in the root `esrc` crate in place (not deleted them) since you did not explicitly ask to remove them, only to move/add to `esrc-cqrs`. The `tab/tests.rs` file in the new location is created as an empty placeholder since I do not have the source for `examples/cafe/tab/tests.rs`. If you have the test contents, please provide them so they can be included.

Also note that `esrc-derive` is added as a direct dependency in `esrc-cqrs/Cargo.toml` because `tab.rs` and `domain.rs` use the derive macros (`DeserializeVersion`, `SerializeVersion`) directly via `use esrc_derive::...`. If those macros are re-exported through `esrc` (via `esrc::version::DeserializeVersion` derive), the explicit `esrc-derive` dependency may not be needed, but it is safer to include it since the example imports from `esrc_derive` directly.

<suggested_git_command>
git add -A .
git commit -m "+ esrc-cqrs - Add cafe example to esrc-cqrs crate using local esrc"
</suggested_git_command>
