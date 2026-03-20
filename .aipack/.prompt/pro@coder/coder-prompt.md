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
  - derive/**/*.*
  - src/**/*.*
  - examples/**/*.*

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

when a command return a error in esrc-cqrs crate the error is "CommandReply".

- the command dispatcher should parse the esrc from the execution of the command, and construct a new esrc_cqrs::Error enum.
  -- One of the variants will be the content of the esrc::External variant. but it should be the serialized content of the Aggregate error.
  -- If the error from the aggregate can't be serializable it should panic the application.
  -- Other variants should be serializable versions of the esrc::Error.

- The user calling the command with the framework should receive the esrc_cqrs::Error.
  -- The user may not know the error stored in the esrc_cqrs::Error::External(e) variant with only with the type signature, but he knows which aggregate the command he execute is, so he know the error enum associated with that aggregate.
  -- He should be able to parse it to a Aggregate error.
  -- modify your command error test case to do that, and validate if the error returned is same you returned from the command.

====
>   Info: Duration: 1m 12s | ~$0.1219 | Prompt Tokens: 15,967 (cached: 0, cache_creation: 0) | Completion Tokens: 4,936 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 12 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the code, I need to:

1. Create a `esrc_cqrs::Error` enum that mirrors `esrc::Error` variants but is serializable, with a special `External` variant for aggregate errors.
2. Modify the command dispatcher to parse `esrc::Error` and construct `esrc_cqrs::Error`, serializing aggregate errors for the `External` variant.
3. Modify `CommandReply` to carry `esrc_cqrs::Error` on failure instead of a plain string.
4. Allow callers to parse the `External` variant back into their aggregate error type.
5. Update the test to validate the error roundtrip.

Let me trace the flow:
- `AggregateCommandHandler::handle` returns `error::Result<Vec<u8>>` where `Vec<u8>` is a serialized `CommandReply`.
- The dispatcher currently encodes failures as `CommandReply { success: false, message: Some(...) }`.
- I need to instead encode a serialized `esrc_cqrs::Error` in the reply.

The `esrc::Error::External` wraps a `BoxStdError` which comes from the aggregate's `Error` type (via `try_write`). The aggregate error must be serializable to JSON.

Key design decisions:
- `esrc_cqrs::Error` is a serializable enum mirroring `esrc::Error`.
- `External` variant stores `serde_json::Value` (the serialized aggregate error).
- `CommandReply` on failure carries the `esrc_cqrs::Error`.
- Caller can do `reply.error().and_then(|e| e.external::<MyAggError>())`.



A few notes on the design:

- `esrc_cqrs::Error` is a fully serializable enum that mirrors `esrc::Error` variants. It lives in `crates/esrc-cqrs/src/error.rs`.
- For the `External` variant, the aggregate's error is serialized into a `serde_json::Value` inside `convert_esrc_error`. This requires `A::Error: Serialize` as a new bound on `AggregateCommandHandler`'s `CommandHandler` impl. If the aggregate error is not serializable, the code panics at the call site, flagging it as a programming error.
- `CommandReply.message: Option<String>` is replaced with `error: Option<cqrs_error::Error>` so callers always receive structured error data.
- `Error::downcast_external::<E>()` lets the caller (who knows which aggregate they targeted) recover the typed error. If the JSON does not match `E`, it panics (programming error).
- The test adds `Serialize`/`Deserialize` for `CounterError` and validates the full roundtrip: send `AlwaysFail`, receive `CommandReply`, downcast to `CounterError::ForcedFailure`.

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs - Add serializable CqrsError and structured aggregate error propagation in CommandReply"
</suggested_git_command>
