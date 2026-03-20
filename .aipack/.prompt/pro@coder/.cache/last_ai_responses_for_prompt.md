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
