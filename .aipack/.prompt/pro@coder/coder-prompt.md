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
improve the integration tests in the esrc-cqrs crate.
they should be more modular and use a unique stream prefix to avoid the stream spam problem i have now.

====
>   Info: Duration: 1m 13s | ~$0.1591 | Prompt Tokens: 24,868 (cached: 0, cache_creation: 0) | Completion Tokens: 5,633 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 17 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the integration tests, the main issues are:

1. Each test leaks a new `&'static str` prefix and creates a brand new NATS stream, leading to stream spam.
2. There is no shared setup/teardown to clean up streams after tests.
3. The `make_store` and `leak` helpers are duplicated and the test structure is not very modular.

The improvement plan:

- Use a single shared stream prefix per test run (via a module-level constant or a once-cell), with each test getting a unique *consumer/subject* prefix derived from that shared stream. This avoids creating a new JetStream stream per test.
- Actually, since NATS JetStream streams are keyed by subject prefixes, the cleanest approach is to create **one stream** per test binary run and use per-test subject prefixes as filter subjects within that stream, cleaning up (purging) after each test.
- Extract a `TestCtx` struct that holds the store, client, and a cleanup handle.
- Use `tokio::test` with a setup helper that provisions and tears down resources.



Key improvements made:

- **`TestCtx` struct** centralises store, client, prefix, and JetStream context. Its `cleanup()` method deletes the stream after each test, preventing stream accumulation.
- **`service_name()` and `durable_name()`** are derived from the unique per-test prefix, so all names stay consistent and unique without extra `leak` calls scattered across tests.
- **`spawn_dispatcher()`** helper extracts the repeated dispatcher-spawning boilerplate into one place.
- **`send_command()`** helper removes the repeated serialise/request/deserialise pattern from every test body.
- Each test now ends with `ctx.cleanup().await`, deleting its dedicated NATS stream so no streams accumulate between runs.
- The random suffix in `TestCtx::new` still ensures parallel test runs do not collide.

<suggested_git_command>
git commit -a -m "^ esrc-cqrs/tests - Modular TestCtx with stream cleanup, shared helpers to reduce stream spam"
</suggested_git_command>
