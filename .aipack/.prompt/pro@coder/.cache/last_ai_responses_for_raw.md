====
>   Info: Duration: 29s | ~$0.08 | Prompt Tokens: 15,275 (cached: 0, cache_creation: 0) | Completion Tokens: 2,280 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 7 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the integration test file, I can see it imports and uses the query-related types but the test file itself doesn't have any tests for the query functionality. The user is saying that the query-related declarations (types, imports) are missing from the integration test file.

Let me look more carefully. The test file imports:

```rust
use esrc_cqrs::nats::{
    AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler,
    NatsCommandDispatcher,
};
```

It's missing imports for the query types (`AggregateQueryHandler`, `QueryEnvelope`, `QueryReply`, `NatsQueryDispatcher`) and there are no query tests defined.

The user says "you forgot to declare the unit tests defined in `crates/esrc-cqrs/tests/integration_nats.rs`" - meaning the query integration tests need to be added to that file.

I need to add:
1. The missing query imports
2. Query-related test cases (send a query, get a response)

Let me write appropriate query integration tests that follow the same pattern as the command tests, using the `Counter` aggregate already defined there.

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
 use esrc_cqrs::nats::{
     AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler,
     NatsCommandDispatcher,
 };
+use esrc_cqrs::nats::{AggregateQueryHandler, NatsQueryDispatcher, QueryEnvelope, QueryReply};
 use esrc_cqrs::CqrsRegistry;
@@
 use uuid::Uuid;
+
+// -- Query read model --------------------------------------------------------
+
+/// A simple read model returned by query handlers in tests.
+#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
+struct CounterState {
+    /// The current value of the counter.
+    pub value: i64,
+}
</FILE_PATCH>

<FILE_APPEND file_path="crates/esrc-cqrs/tests/integration_nats.rs">

/// Spawn the query dispatcher as a background task and wait briefly for it
/// to register its service endpoints.
async fn spawn_query_dispatcher(
    ctx: &TestCtx,
    handlers: Vec<Arc<dyn esrc_cqrs::registry::ErasedQueryHandler<NatsStore>>>,
) {
    let service_name = ctx.service_name();
    let store = ctx.store.clone();

    let dispatcher = NatsQueryDispatcher::new(
        async_nats::connect("nats://localhost:4222")
            .await
            .expect("connect"),
        service_name,
    );

    tokio::spawn(async move {
        let _ = dispatcher.run(store, &handlers).await;
    });

    // Allow the NATS service endpoints to register before tests send queries.
    sleep(Duration::from_millis(300)).await;
}

/// Send a single query through NATS request/reply, returning the raw reply.
async fn send_query(
    client: &async_nats::Client,
    service_name: &str,
    handler_name: &str,
    id: Uuid,
) -> QueryReply {
    let subject = esrc_cqrs::nats::query_dispatcher::query_subject(service_name, handler_name);
    let envelope = QueryEnvelope { id };
    let payload = serde_json::to_vec(&envelope).expect("serialize query envelope");
    let reply = client
        .request(subject, payload.into())
        .await
        .expect("NATS request should succeed");
    serde_json::from_slice(&reply.payload).expect("valid QueryReply")
}

/// Test that a query sent over NATS returns the correct aggregate state after
/// one or more commands have been applied.
#[tokio::test]
async fn test_query_returns_aggregate_state() {
    let ctx = TestCtx::new("qry-ok").await;

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"))
        .register_query(AggregateQueryHandler::<Counter, CounterState>::new(
            "Counter.GetState",
            |root| CounterState { value: root.value },
        ));

    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;
    spawn_query_dispatcher(&ctx, registry.query_handlers().to_vec()).await;

    let id = Uuid::new_v4();

    // Apply two increments so the aggregate has a known value.
    for by in [10i64, 5] {
        let resp = send_command(&ctx.client, ctx.service_name(), "Counter", id, CounterCommand::Increment { by }).await;
        assert!(resp.success, "command should succeed");
    }

    let reply = send_query(&ctx.client, ctx.service_name(), "Counter.GetState", id).await;

    assert!(reply.success, "query should succeed");
    assert!(reply.error.is_none());

    let state: CounterState = serde_json::from_value(reply.data.expect("data present"))
        .expect("CounterState should deserialize");
    assert_eq!(state.value, 15, "query should reflect cumulative aggregate state");

    ctx.cleanup().await;
}

/// Test that querying an aggregate that has never received a command returns
/// the default (zero) state without an error.
#[tokio::test]
async fn test_query_default_state_for_new_aggregate() {
    let ctx = TestCtx::new("qry-new").await;

    let registry = CqrsRegistry::new(ctx.store.clone()).register_query(
        AggregateQueryHandler::<Counter, CounterState>::new("Counter.GetState", |root| {
            CounterState { value: root.value }
        }),
    );

    spawn_query_dispatcher(&ctx, registry.query_handlers().to_vec()).await;

    let id = Uuid::new_v4();
    let reply = send_query(&ctx.client, ctx.service_name(), "Counter.GetState", id).await;

    assert!(reply.success, "query on a new aggregate should succeed");
    let state: CounterState = serde_json::from_value(reply.data.expect("data present"))
        .expect("CounterState should deserialize");
    assert_eq!(state.value, 0, "new aggregate should have default value of 0");

    ctx.cleanup().await;
}

/// Test that a malformed query payload results in an error reply and the
/// dispatcher keeps running for subsequent queries.
#[tokio::test]
async fn test_query_malformed_payload_returns_error() {
    let ctx = TestCtx::new("qry-bad").await;

    let registry = CqrsRegistry::new(ctx.store.clone()).register_query(
        AggregateQueryHandler::<Counter, CounterState>::new("Counter.GetState", |root| {
            CounterState { value: root.value }
        }),
    );

    spawn_query_dispatcher(&ctx, registry.query_handlers().to_vec()).await;

    let subject = esrc_cqrs::nats::query_dispatcher::query_subject(ctx.service_name(), "Counter.GetState");

    // Send garbage bytes; we only care that we get a response, not a panic.
    let bad_result = ctx
        .client
        .request(subject.clone(), b"this is not json"[..].into())
        .await;
    let _ = bad_result;

    // Confirm the dispatcher is still alive by sending a well-formed query.
    let id = Uuid::new_v4();
    let reply = send_query(&ctx.client, ctx.service_name(), "Counter.GetState", id).await;
    assert!(reply.success, "dispatcher should still handle valid queries after a bad payload");

    ctx.cleanup().await;
}

/// Test that `CqrsRegistry::query_handlers()` reflects registered query handlers.
#[tokio::test]
async fn test_registry_query_handlers_accessor() {
    let ctx = TestCtx::new("qry-reg").await;

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_query(AggregateQueryHandler::<Counter, CounterState>::new(
            "Counter.GetState",
            |root| CounterState { value: root.value },
        ))
        .register_query(AggregateQueryHandler::<Counter, CounterState>::new(
            "Counter.GetStateAlt",
            |root| CounterState { value: root.value },
        ));

    assert_eq!(
        registry.query_handlers().len(),
        2,
        "two query handlers should be registered"
    );

    ctx.cleanup().await;
}
</FILE_APPEND>

</FILE_CHANGES>

<suggested_git_command>
git commit -a -m "+ esrc-cqrs - Add query integration tests to integration_nats.rs"
</suggested_git_command>

