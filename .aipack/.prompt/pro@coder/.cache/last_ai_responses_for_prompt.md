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
