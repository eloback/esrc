```yaml
#!meta (parametric prompt)

# See PATH_TO_PRO_CODER_DIR/README.md for additional documentation
# Note: All relative paths are relative to the workspace directory (parent folder of .aipack/)

## Static knowledge (relative & absolute)
knowledge_globs:
  # - /abs/or/rel/path/to/**/*.md      # Any relative or absolute path/glob for Markdown files
  - nats_service.md
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
context_globs_post:        # Appended after auto-context selection
  - .aipack/.prompt/pro@coder/dev/plan/*.md  

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - derive/**/*.*
  - src/**/*.*
  - example/cafe/**/*.*
  
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
Following the plan rules, execute the next step in the plan and update the appropriate files.

====
>   Info: Duration: 35s | ~$0.1503 | Prompt Tokens: 35,221 (cached: 0, cache_creation: 0) | Completion Tokens: 2,977 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 38 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll execute the next step from the todo plan: "Create the cafe example skeleton with domain types".



The domain skeleton for the cafe example is now in place. Here is a summary of what was created and changed:

- `examples/cafe/domain.rs`: defines `Order` (aggregate), `OrderStatus`, `OrderCommand`, `OrderEvent` (with derive macros for `Event`, `SerializeVersion`, `DeserializeVersion`, `Serialize`, `Deserialize`), `OrderError`, and `impl Aggregate for Order`.
- `examples/cafe/main.rs`: minimal entry point with a module declaration for `domain`, prints a confirmation message.
- `Cargo.toml`: added `[[example]]` entry for `cafe` pointing at `examples/cafe/main.rs` with `required-features = ["nats", "derive"]`.
- Plan files updated: previous active step moved to done, new step activated.

<suggested_git_commit>
git add -A .
git commit -a -m "+ cafe example - Add domain skeleton with Order aggregate, events, and commands"
</suggested_git_commit>
