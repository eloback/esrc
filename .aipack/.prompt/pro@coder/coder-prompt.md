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
the user calling the command should receive instead the esrc::Error or a error that wraps it exclusive to the Nats dispatcher.
the user may not know the error stored in the esrc::Error::External(e) variant with only with the type signature, but the user know which command he called and what ErrorEnum would be mapped to that command, so he should be able to parse it, modify your command error test case to do that, and validate if the error returned is same you responded with.

====
>   Info: Duration: 1m 25s | ~$0.1399 | Prompt Tokens: 12,098 (cached: 0, cache_creation: 0) | Completion Tokens: 6,907 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 7 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the code, the issue is that when a command fails, the dispatcher returns a `CommandReply` with `success: false` and a string message, losing all type information about the error. The caller cannot recover the original error type.

The fix is to:

1. Change `CommandReply` to carry structured error information (serialized error bytes + a type hint) instead of just a string message.
2. Have the `AggregateCommandHandler` serialize the error payload (the `External` boxed error downcasted, or the full `esrc::Error` as JSON).
3. Expose a way for the caller to deserialize the original error back from the reply.
4. Update the test to deserialize the `CounterError` from the reply and assert equality.

The cleanest approach: `CommandReply` carries an optional `error_payload` as raw JSON bytes (base64 or serde `Value`), so callers who know the error type can deserialize it. The `AggregateCommandHandler` serializes the `esrc::Error` using a wrapper that includes the variant name and the inner error as JSON.

Let me design the `CommandError` type that wraps `esrc::Error` in a serializable form:

- `variant`: string like `"External"`, `"Internal"`, `"Conflict"`, etc.
- `payload`: optional `serde_json::Value` with the inner error serialized (only meaningful for `External` where the user's error type lives).

The `CommandReply` will carry `Option<CommandError>` instead of `Option<String>`.



Let me redo the patches properly without the conflicting hunks:



Here is a summary of the changes:

- **`CommandReply.message: Option<String>`** is replaced by **`CommandReply.error: Option<CommandError>`**. The new `CommandError` type carries a `variant` string (the `esrc::Error` arm name) and a `payload: serde_json::Value`.

- **`CommandError`** is a new public type. The `variant` field tells the caller which `esrc::Error` arm fired. For `"External"` (aggregate domain errors), `payload["message"]` holds the `Display` text of the concrete error, which the caller can match against their known error type's display strings since they know what command they sent.

- **`esrc_error_to_command_error`** is the private helper that maps each `esrc::Error` variant to the wire representation.

- The **dispatcher fallback** (parse/handler errors) uses `variant: "Internal"` consistently.

- The **test** now sends `AlwaysFail`, receives a `CommandReply`, asserts `success == false`, extracts `CommandError`, checks `variant == "External"`, and compares `payload["message"]` against `CounterError::ForcedFailure.to_string()`, which is the text the caller would know from having called that specific command.

<suggested_git_command>
git add -A .
git commit -a -m "^ esrc-cqrs - Replace CommandReply.message with typed CommandError carrying esrc::Error variant and JSON payload"
</suggested_git_command>
