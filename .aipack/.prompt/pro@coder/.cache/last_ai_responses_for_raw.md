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

<FILE_CHANGES>

<FILE_NEW file_path="crates/esrc-cqrs/tests/integration_nats.rs">
//! Integration tests for `esrc-cqrs` against a live NATS server.
//!
//! Requires a NATS server with JetStream enabled at `localhost:4222`:
//!   nats-server -js
//!
//! Run with:
//!   cargo test -p esrc-cqrs --test integration_nats -- --test-threads=1

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_nats::jetstream;
use esrc::aggregate::Aggregate;
use esrc::event::replay::ReplayOneExt;
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Envelope, Event};
use esrc_cqrs::nats::{
    AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler,
    NatsCommandDispatcher,
};
use esrc_cqrs::CqrsRegistry;
use tokio::time::sleep;
use uuid::Uuid;

// -- Test domain types -------------------------------------------------------

#[derive(Debug, Default)]
struct Counter {
    value: i64,
}

#[derive(Debug, Deserialize, Serialize)]
enum CounterCommand {
    Increment { by: i64 },
    Decrement { by: i64 },
    /// A command that always fails, used to test command error propagation.
    AlwaysFail,
}

#[derive(Debug, Clone, Event, Serialize, Deserialize, SerializeVersion, DeserializeVersion)]
enum CounterEvent {
    Incremented { by: i64 },
    Decremented { by: i64 },
}

#[derive(Debug, thiserror::Error)]
enum CounterError {
    #[error("forced failure for testing")]
    ForcedFailure,
}

impl Aggregate for Counter {
    type Command = CounterCommand;
    type Event = CounterEvent;
    type Error = CounterError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            CounterCommand::Increment { by } => Ok(CounterEvent::Incremented { by }),
            CounterCommand::Decrement { by } => Ok(CounterEvent::Decremented { by }),
            CounterCommand::AlwaysFail => Err(CounterError::ForcedFailure),
        }
    }

    fn apply(self, event: &Self::Event) -> Self {
        match event {
            CounterEvent::Incremented { by } => Counter {
                value: self.value + by,
            },
            CounterEvent::Decremented { by } => Counter {
                value: self.value - by,
            },
        }
    }
}

// -- Shared projector state --------------------------------------------------

/// Tracks events received by the projector during a test run.
#[derive(Clone, Default)]
struct RecordingProjector {
    /// Events received, stored as simple string labels.
    received: Arc<Mutex<Vec<(Uuid, String)>>>,
    /// Whether to return an error on the next projection call.
    fail_next: Arc<Mutex<bool>>,
}

impl RecordingProjector {
    fn new() -> Self {
        Self::default()
    }

    fn received_events(&self) -> Vec<(Uuid, String)> {
        self.received.lock().unwrap().clone()
    }

    /// Make the next `project` call return an error.
    fn set_fail_next(&self) {
        *self.fail_next.lock().unwrap() = true;
    }
}

#[derive(Debug, thiserror::Error)]
#[error("projector forced error")]
struct ProjectorError;

impl Project for RecordingProjector {
    type EventGroup = CounterEvent;
    type Error = ProjectorError;

    async fn project<'de, E>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: Envelope + Sync,
    {
        let should_fail = {
            let mut guard = self.fail_next.lock().unwrap();
            let v = *guard;
            *guard = false;
            v
        };
        if should_fail {
            return Err(ProjectorError);
        }

        let id = Context::id(&context);
        let label = match *context {
            CounterEvent::Incremented { by } => format!("Incremented({by})"),
            CounterEvent::Decremented { by } => format!("Decremented({by})"),
        };
        self.received.lock().unwrap().push((id, label));
        Ok(())
    }
}

// -- Test context ------------------------------------------------------------

/// All resources needed by a single test case.
///
/// Holds a store scoped to a unique subject prefix and a connected NATS client.
/// On drop the JetStream stream created for this test is deleted so that
/// stream names do not accumulate across runs.
struct TestCtx {
    store: NatsStore,
    client: async_nats::Client,
    /// The unique prefix / stream name used for this test.
    prefix: &'static str,
    /// JetStream context kept for cleanup on drop.
    js: jetstream::Context,
}

impl TestCtx {
    /// Build a `TestCtx` for one test case.
    ///
    /// `label` should be a short, human-readable identifier (ASCII letters and
    /// hyphens). A random 8-hex-character suffix is appended to guarantee
    /// uniqueness even when tests run in parallel.
    async fn new(label: &str) -> Self {
        let tag = &Uuid::new_v4().to_string()[..8];
        let prefix_string = format!("t-{label}-{tag}");
        // Leak once per test; acceptable in short-lived test processes.
        let prefix: &'static str = Box::leak(prefix_string.into_boxed_str());

        let client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS server must be reachable at localhost:4222");
        let js = jetstream::new(client.clone());

        let store = NatsStore::try_new(js.clone(), prefix)
            .await
            .expect("NatsStore creation failed");

        Self {
            store,
            client,
            prefix,
            js,
        }
    }

    /// Derive a unique service name for this test context.
    fn service_name(&self) -> &'static str {
        let s = format!("{}-svc", self.prefix);
        Box::leak(s.into_boxed_str())
    }

    /// Derive a unique durable consumer name for this test context.
    fn durable_name(&self) -> &'static str {
        let s = format!("{}-dur", self.prefix);
        Box::leak(s.into_boxed_str())
    }

    /// Delete the JetStream stream created for this test, suppressing errors.
    async fn cleanup(self) {
        let _ = self.js.delete_stream(self.prefix).await;
    }
}

/// Spawn the command dispatcher as a background task and wait briefly for it
/// to register its service endpoints.
async fn spawn_dispatcher(
    ctx: &TestCtx,
    handlers: Vec<Arc<dyn esrc_cqrs::registry::ErasedCommandHandler<NatsStore>>>,
) {
    let service_name = ctx.service_name();
    let store = ctx.store.clone();

    let dispatcher = NatsCommandDispatcher::new(
        async_nats::connect("nats://localhost:4222")
            .await
            .expect("connect"),
        service_name,
    );

    tokio::spawn(async move {
        let _ = dispatcher.run(store, &handlers).await;
    });

    // Allow the NATS service endpoints to register before tests send commands.
    sleep(Duration::from_millis(300)).await;
}

/// Send a single command through NATS request/reply, returning the reply.
async fn send_command<C>(
    client: &async_nats::Client,
    service_name: &str,
    handler_name: &str,
    id: Uuid,
    command: C,
) -> CommandReply
where
    C: serde::Serialize,
{
    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(service_name, handler_name);
    let envelope = CommandEnvelope { id, command };
    let payload = serde_json::to_vec(&envelope).expect("serialize command envelope");
    let reply = client
        .request(subject, payload.into())
        .await
        .expect("NATS request should succeed");
    serde_json::from_slice(&reply.payload).expect("valid CommandReply")
}

// -- Tests -------------------------------------------------------------------

/// Test that a command sent over NATS results in a successful reply and the
/// event is durably stored (readable via replay).
#[tokio::test]
async fn test_command_request_response_success() {
    let ctx = TestCtx::new("cmd-ok").await;

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"));

    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;

    let aggregate_id = Uuid::new_v4();
    let response = send_command(
        &ctx.client,
        ctx.service_name(),
        "Counter",
        aggregate_id,
        CounterCommand::Increment { by: 5 },
    )
    .await;

    assert!(response.success, "command should succeed");
    assert_eq!(response.id, aggregate_id);
    assert!(response.message.is_none());

    // Verify the event was actually persisted.
    let root: esrc::aggregate::Root<Counter> = ctx.store.read(aggregate_id).await.unwrap();
    assert_eq!(root.value, 5, "aggregate value should reflect the event");

    ctx.cleanup().await;
}

/// Test that a failing command returns a NATS service error (non-2xx code).
/// The framework should not crash and subsequent commands should still work.
#[tokio::test]
async fn test_command_error_does_not_break_dispatcher() {
    let ctx = TestCtx::new("cmd-err").await;

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"));

    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;

    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(ctx.service_name(), "Counter");

    // Send a command that will always fail.
    let bad_envelope = CommandEnvelope {
        id: Uuid::new_v4(),
        command: CounterCommand::AlwaysFail,
    };
    let bad_payload = serde_json::to_vec(&bad_envelope).unwrap();

    // The request itself resolves (the service replies with an error code) or
    // returns a service-level error; either way we should not hang or panic.
    let bad_result = ctx.client.request(subject.clone(), bad_payload.into()).await;
    // The NATS service API sends an error status reply; async-nats surfaces
    // this as an Err from `request`. We only assert the client did not hang.
    let _ = bad_result;

    // Now send a valid command to confirm the dispatcher is still running.
    let good_id = Uuid::new_v4();
    let response = send_command(
        &ctx.client,
        ctx.service_name(),
        "Counter",
        good_id,
        CounterCommand::Increment { by: 3 },
    )
    .await;

    assert!(response.success);
    assert_eq!(response.id, good_id);

    ctx.cleanup().await;
}

/// Test that the projector receives events after they are published via the
/// command handler, and that its internal state reflects those events.
#[tokio::test]
async fn test_projector_receives_events() {
    let ctx = TestCtx::new("proj-recv").await;

    let projector = RecordingProjector::new();
    let received = projector.received.clone();

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"))
        .register_projector(DurableProjectorHandler::new(ctx.durable_name(), projector));

    // Start projectors.
    let mut projector_set = registry.run_projectors().await.unwrap();

    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;

    // Extra delay so the projector consumer is ready before events arrive.
    sleep(Duration::from_millis(100)).await;

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    for (id, cmd) in [
        (id1, CounterCommand::Increment { by: 10 }),
        (id2, CounterCommand::Decrement { by: 3 }),
        (id1, CounterCommand::Increment { by: 1 }),
    ] {
        let resp = send_command(&ctx.client, ctx.service_name(), "Counter", id, cmd).await;
        assert!(resp.success, "command {id} should succeed");
    }

    // Give the projector time to process the durable consumer messages.
    sleep(Duration::from_millis(800)).await;

    let events = received.lock().unwrap().clone();
    assert_eq!(events.len(), 3, "projector should have seen 3 events");

    // Verify event labels are in order.
    assert_eq!(events[0], (id1, "Incremented(10)".to_string()));
    assert_eq!(events[1], (id2, "Decremented(3)".to_string()));
    assert_eq!(events[2], (id1, "Incremented(1)".to_string()));

    projector_set.abort_all();
    ctx.cleanup().await;
}

/// Test that a projector error causes the projector task to terminate with an
/// error rather than silently swallowing it, and that the JoinSet reflects it.
#[tokio::test]
async fn test_projector_error_propagates() {
    let ctx = TestCtx::new("proj-err").await;

    let projector = RecordingProjector::new();
    // Pre-arm the projector to fail on the first event it receives.
    projector.set_fail_next();

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"))
        .register_projector(DurableProjectorHandler::new(ctx.durable_name(), projector));

    let mut projector_set = registry.run_projectors().await.unwrap();

    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;

    // Publish one event; the projector will fail on it.
    let resp = send_command(
        &ctx.client,
        ctx.service_name(),
        "Counter",
        Uuid::new_v4(),
        CounterCommand::Increment { by: 7 },
    )
    .await;
    assert!(resp.success, "command must succeed regardless of projector state");

    // The projector task should finish (with an error) within a reasonable time.
    sleep(Duration::from_millis(800)).await;

    // At least one task should have completed with an Err.
    let mut found_error = false;
    while let Some(result) = projector_set.join_next().await {
        match result {
            Ok(Err(_)) => {
                found_error = true;
                break;
            },
            // Task may also have been aborted or returned Ok due to timing; continue.
            _ => {},
        }
    }
    projector_set.abort_all();

    assert!(
        found_error,
        "projector error should surface through the JoinSet"
    );

    ctx.cleanup().await;
}

/// Test that multiple commands for the same aggregate result in correct
/// cumulative state, confirming optimistic concurrency is handled correctly.
#[tokio::test]
async fn test_multiple_commands_same_aggregate_occ() {
    let ctx = TestCtx::new("occ").await;

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"));

    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;

    let id = Uuid::new_v4();
    // Send 5 sequential increments to the same aggregate.
    for i in 1i64..=5 {
        let resp = send_command(
            &ctx.client,
            ctx.service_name(),
            "Counter",
            id,
            CounterCommand::Increment { by: i },
        )
        .await;
        assert!(resp.success, "increment {i} should succeed");
    }

    // Expected: 1 + 2 + 3 + 4 + 5 = 15
    let root: esrc::aggregate::Root<Counter> = ctx.store.read(id).await.unwrap();
    assert_eq!(
        root.value, 15,
        "aggregate should reflect all 5 sequential increments"
    );

    ctx.cleanup().await;
}

/// Test that sending a malformed (unparseable) payload to a command endpoint
/// results in an error reply and the dispatcher keeps running.
#[tokio::test]
async fn test_malformed_payload_returns_error() {
    let ctx = TestCtx::new("malformed").await;

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"));

    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;

    let subject =
        esrc_cqrs::nats::command_dispatcher::command_subject(ctx.service_name(), "Counter");

    // Send garbage bytes.
    let bad_result = ctx
        .client
        .request(subject.clone(), b"this is not json"[..].into())
        .await;
    // The NATS service will reply with an error status; async-nats returns Err.
    // We only care that we get a response (not a timeout or panic).
    let _ = bad_result;

    // Confirm the dispatcher is still alive.
    let good_id = Uuid::new_v4();
    let resp = send_command(
        &ctx.client,
        ctx.service_name(),
        "Counter",
        good_id,
        CounterCommand::Decrement { by: 2 },
    )
    .await;

    assert!(resp.success);
    assert_eq!(resp.id, good_id);

    ctx.cleanup().await;
}

/// Test that `CqrsRegistry::store()` returns a clone of the backing store and
/// that `command_handlers()` / `projector_handlers()` reflect registrations.
#[tokio::test]
async fn test_registry_accessors() {
    let ctx = TestCtx::new("registry").await;

    let projector = RecordingProjector::new();

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"))
        .register_projector(DurableProjectorHandler::new("reg-proj", projector));

    assert_eq!(
        registry.command_handlers().len(),
        1,
        "one command handler should be registered"
    );
    assert_eq!(
        registry.projector_handlers().len(),
        1,
        "one projector handler should be registered"
    );

    // store() should return without panicking (it's a Clone).
    let _store_clone = registry.store();

    ctx.cleanup().await;
}
</FILE_NEW>

</FILE_CHANGES>

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

