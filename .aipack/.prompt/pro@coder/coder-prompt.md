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
  - nats_service.md

## Files the AI will work on (paths & content included in prompt, relative only)
context_globs:
  - Cargo.toml # for Rust
  - src/**/*
  - examples/cafe/**/*
  # - crates/esrc-cqrs/**/*
  # - derive/**/*.*
  # - compilation_errors.txt

#context_globs_post: # Appended after auto-context selection
# - .aipack/.prompt/pro@coder/dev/plan/*.md

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
  chat: _workbench/service # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  plan: _workbench/service # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: opus
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

Update the cafe example to use the new command service feature, and remove the old esrc-cqrs dependecy

====
>   Info: Duration: 37s | ~$0.1725 | Prompt Tokens: 22,015 (cached: 0, cache_creation: 0) | Completion Tokens: 2,497 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 14 | Working Files: 0 | Knowledge Files: 2
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the current state, the plan-3-done-steps.md shows the last step "Update cafe example to use native esrc CommandService, remove esrc-cqrs dependency" is already marked as done. However, looking at the actual `examples/cafe/main.rs` code, it still heavily references `esrc_cqrs` types (`CqrsClient`, `NatsCommandDispatcher`, `NatsQueryDispatcher`, `CqrsRegistry`, etc.) and the `projector.rs` file still exists.

Let me implement what the done step describes: rewrite the cafe example to use the native `esrc` `CommandService`/`spawn_service`, remove the `esrc-cqrs` dependency, and clean up accordingly.

I also need to check what files reference `esrc-cqrs`. The `examples/cafe/main.rs` imports from `esrc_cqrs` and references a `service` module. Let me also look at domain.rs which has `OrderState` and `View` impl that should be removed per the done step notes. The `projector.rs` should be deleted.



The changes made:

1. **`examples/cafe/main.rs`** - Completely rewritten to use the native `esrc` `CommandService` via `store.spawn_service::<Order>()`. Removed all `esrc_cqrs` imports and types (`CqrsClient`, `NatsCommandDispatcher`, `NatsQueryDispatcher`, `CqrsRegistry`, etc.). Commands are sent via direct NATS `request` calls to the command service endpoint subject (`<event_name>.command.<uuid>`). Queries are handled via direct `store.read::<Order>(id)` replay calls.

2. **`examples/cafe/domain.rs`** - Removed `OrderState`, `View` import, and the `View` impl, since queries are now handled via direct aggregate replay. Also removed the unused `view::View` import.

3. **`examples/cafe/projector.rs`** - Deleted, as projector functionality is no longer used in this example.

Note: The `Cargo.toml` example entry already has `required-features = ["nats", "derive"]` which is correct for the updated example. If there was an `esrc-cqrs` dependency in the workspace or example `Cargo.toml`, that would also need removal, but it's not present in the provided `Cargo.toml`.

<suggested_git_command>
git add -A . && git commit -m "^ cafe example - Rewrite to use native esrc CommandService, remove esrc-cqrs dependency"
</suggested_git_command>
