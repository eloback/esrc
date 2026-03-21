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
>   Info: Duration: 7s 201ms | ~$0.11 | Prompt Tokens: 35,273 (cached: 0, cache_creation: 0) | Completion Tokens: 278 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 35 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I need to move the cafe example from the `esrc` crate to the `esrc-cqrs` crate. Let me analyze what's needed:

1. The cafe example files need to move to `crates/esrc-cqrs/examples/cafe/`
2. The `esrc-cqrs` `Cargo.toml` needs updating to add the example, dev-dependencies, and the local `esrc` dependency with the right features
3. The root `Cargo.toml` needs the example entry removed (or updated)
4. The example's imports should still work since `esrc-cqrs` already depends on `esrc`

I need the `crates/esrc-cqrs/Cargo.toml` to see its current state, but I don't have it.

<missing_files>
<mf_message>
I need the esrc-cqrs Cargo.toml to update it with the new example dependencies and entry point. Please provide this file.
</mf_message>
<mf_files>
  - crates/esrc-cqrs/Cargo.toml
</mf_files>
</missing_files>
