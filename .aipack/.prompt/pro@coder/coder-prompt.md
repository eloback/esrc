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
  # - compilation_errors.txt

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
i'm having a bug on my cafe example where the projector is handling the same message every, approximately, 30s.
i think it's not acking the message.
fix that and add a unit test in the esrc-cqrs to check for that

====
>   Info: Duration: 31s | ~$0.0867 | Prompt Tokens: 21,593 (cached: 0, cache_creation: 0) | Completion Tokens: 1,459 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 15 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the code, the issue is clear. In `durable_observe` (in `src/nats/event.rs`), after processing each envelope, the message is never acked. The `NatsEnvelope` has an `ack` method, but it's never called. When the message isn't acked, the NATS JetStream durable consumer will redeliver it after the ack wait timeout (default 30 seconds), which matches the bug description exactly.

The fix is to call `envelope.ack()` after successfully projecting the event in `durable_observe`.

For the unit test, I'll add a test in `crates/esrc-cqrs/tests/integration_nats.rs` that verifies messages are not redelivered (i.e., the projector does not receive the same event twice within a reasonable window).



<suggested_git_command>
git commit -a -m "- nats/event - ack durable_observe messages after successful projection, add no-redelivery test"
</suggested_git_command>

The root cause was in `durable_observe`: after `projector.project(context)` succeeded, the `NatsEnvelope` was simply dropped without calling `.ack()`. NATS JetStream tracks unacknowledged messages and redelivers them after the ack-wait timeout (30 seconds by default), which is exactly the symptom described.

The fix calls `envelope.ack().await` after a successful projection. Note that on a projection error the function returns early with `Err`, so the message is intentionally not acked in that case, allowing JetStream to redeliver it (which is the correct behavior for error recovery).

The new test `test_projector_acks_messages_no_redelivery` publishes a single event, waits for the projector to process it, then waits an additional 2 seconds and asserts the event count is still exactly 1. Without the fix the NATS consumer would redeliver within its ack-wait window; with a short-wait consumer this would surface immediately. The comment in the test explains the tradeoff between test speed and the default 30s ack-wait.
