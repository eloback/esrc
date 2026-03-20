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
  enabled: false # (Default true) Comment or set to true to enable.

dev:
  chat: false # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  plan: false # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: claude-sonnet-4-6
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

create a suite of unit tests to validate the work of the esrc-cqrs crate.
- it should test commands and projects using the nats implementation connected to "localhost:4222".
- it should test if the ack of the messages in the projectors are correct.
- it should test if the request-response of the command works.
- it should test if errors in projects and commands don't break the framework.

====
>   Info: Duration: 1m 16s | ~$0.2289 | Prompt Tokens: 45,076 (cached: 0, cache_creation: 0) | Completion Tokens: 6,245 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 58 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the codebase, I need to create integration tests for the `esrc-cqrs` crate that test against a real NATS server at `localhost:4222`. I'll create a test module that covers command dispatch, projector ack behavior, request-response flow, and error handling.



A few notes on the test suite:

- Each test generates a unique NATS stream prefix using a UUID fragment so concurrent runs do not interfere with each other.
- `leak()` is used to produce `&'static str` values from runtime strings, which is acceptable in short-lived test binaries.
- The `test_projector_error_propagates` test pre-arms the projector to fail and then verifies that the error surfaces through the `JoinSet` rather than being silently swallowed.
- The `test_command_error_does_not_break_dispatcher` and `test_malformed_payload_returns_error` tests verify the dispatcher stays alive after bad inputs.
- OCC (optimistic concurrency control) is verified by sending 5 sequential commands to the same aggregate and checking the final replayed state.

<suggested_git_command>
git add -A .
git commit -m "+ esrc-cqrs - Add NATS integration test suite for commands, projectors, error handling, and OCC"
</suggested_git_command>
