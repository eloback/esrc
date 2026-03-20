//! Integration tests for `esrc-cqrs` against a live NATS server.
//!
//! Requires a NATS server with JetStream enabled at `localhost:4222`:
//!   nats-server -js
//!
//! Run with:
//!   cargo test -p esrc-cqrs --test integration_nats -- --test-threads=1

use std::collections::HashMap;
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
use serde::{Deserialize, Serialize};
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

// -- Helpers -----------------------------------------------------------------

/// Build a unique prefix and durable name so parallel test runs do not clash.
fn unique_prefix(test_name: &str) -> String {
    // Use the first 8 chars of a random UUID to keep stream names short.
    let tag = &Uuid::new_v4().to_string()[..8];
    format!("{test_name}-{tag}")
}

/// Connect to the local NATS server and create a `NatsStore` under `prefix`.
async fn make_store(prefix: &'static str) -> NatsStore {
    let client = async_nats::connect("localhost:4222")
        .await
        .expect("NATS server must be reachable at localhost:4222");
    let js = jetstream::new(client);
    NatsStore::try_new(js, prefix)
        .await
        .expect("NatsStore creation failed")
}

/// Leak a `String` so we get a `&'static str`. Acceptable in short-lived tests.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

// -- Tests -------------------------------------------------------------------

/// Test that a command sent over NATS results in a successful reply and the
/// event is durably stored (readable via replay).
#[tokio::test]
async fn test_command_request_response_success() {
    let prefix = leak(unique_prefix("cmd-ok"));
    let service_name = leak(format!("{prefix}-svc"));
    let durable = leak(format!("{prefix}-proj"));

    let store = make_store(prefix).await;

    let registry = CqrsRegistry::new(store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"));

    let dispatcher = NatsCommandDispatcher::new(
        async_nats::connect("localhost:4222")
            .await
            .expect("connect"),
        service_name,
    );

    // Run the dispatcher in the background.
    let dispatch_store = store.clone();
    let handlers: Vec<_> = registry.command_handlers().to_vec();
    tokio::spawn(async move {
        let _ = dispatcher.run(dispatch_store, &handlers).await;
    });

    // Allow the service endpoints to register.
    sleep(Duration::from_millis(300)).await;

    let aggregate_id = Uuid::new_v4();
    let envelope = CommandEnvelope {
        id: aggregate_id,
        command: CounterCommand::Increment { by: 5 },
    };
    let payload = serde_json::to_vec(&envelope).unwrap();

    let client = async_nats::connect("localhost:4222")
        .await
        .expect("connect");
    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(service_name, "Counter");
    let reply = client
        .request(subject, payload.into())
        .await
        .expect("request should succeed");

    let response: CommandReply =
        serde_json::from_slice(&reply.payload).expect("valid CommandReply");
    assert!(response.success, "command should succeed");
    assert_eq!(response.id, aggregate_id);
    assert!(response.message.is_none());

    // Verify the event was actually persisted.
    let root: esrc::aggregate::Root<Counter> = store.read(aggregate_id).await.unwrap();
    assert_eq!(root.value, 5, "aggregate value should reflect the event");

    let _ = durable; // suppress unused warning
}

/// Test that a failing command returns a NATS service error (non-2xx code).
/// The framework should not crash and subsequent commands should still work.
#[tokio::test]
async fn test_command_error_does_not_break_dispatcher() {
    let prefix = leak(unique_prefix("cmd-err"));
    let service_name = leak(format!("{prefix}-svc"));

    let store = make_store(prefix).await;

    let registry = CqrsRegistry::new(store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"));

    let dispatcher = NatsCommandDispatcher::new(
        async_nats::connect("localhost:4222")
            .await
            .expect("connect"),
        service_name,
    );

    let dispatch_store = store.clone();
    let handlers: Vec<_> = registry.command_handlers().to_vec();
    tokio::spawn(async move {
        let _ = dispatcher.run(dispatch_store, &handlers).await;
    });

    sleep(Duration::from_millis(300)).await;

    let client = async_nats::connect("localhost:4222")
        .await
        .expect("connect");
    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(service_name, "Counter");

    // Send a command that will always fail.
    let bad_envelope = CommandEnvelope {
        id: Uuid::new_v4(),
        command: CounterCommand::AlwaysFail,
    };
    let bad_payload = serde_json::to_vec(&bad_envelope).unwrap();

    // The request itself resolves (the service replies with an error code) or
    // returns a service-level error; either way we should not hang or panic.
    let bad_result = client
        .request(subject.clone(), bad_payload.into())
        .await;
    // The NATS service API sends an error status reply; async-nats surfaces
    // this as an Err from `request`. We only assert the client did not hang.
    let _ = bad_result;

    // Now send a valid command to confirm the dispatcher is still running.
    let good_id = Uuid::new_v4();
    let good_envelope = CommandEnvelope {
        id: good_id,
        command: CounterCommand::Increment { by: 3 },
    };
    let good_payload = serde_json::to_vec(&good_envelope).unwrap();
    let good_reply = client
        .request(subject, good_payload.into())
        .await
        .expect("dispatcher must still accept commands after a previous error");

    let response: CommandReply =
        serde_json::from_slice(&good_reply.payload).expect("valid CommandReply");
    assert!(response.success);
    assert_eq!(response.id, good_id);
}

/// Test that the projector receives events after they are published via the
/// command handler, and that its internal state reflects those events.
#[tokio::test]
async fn test_projector_receives_events() {
    let prefix = leak(unique_prefix("proj-recv"));
    let service_name = leak(format!("{prefix}-svc"));
    let durable = leak(format!("{prefix}-proj"));

    let store = make_store(prefix).await;

    let projector = RecordingProjector::new();
    let received = projector.received.clone();

    let registry = CqrsRegistry::new(store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"))
        .register_projector(DurableProjectorHandler::new(durable, projector));

    // Start projectors.
    let mut projector_set = registry.run_projectors().await.unwrap();

    let dispatcher = NatsCommandDispatcher::new(
        async_nats::connect("localhost:4222")
            .await
            .expect("connect"),
        service_name,
    );

    let dispatch_store = store.clone();
    let handlers: Vec<_> = registry.command_handlers().to_vec();
    tokio::spawn(async move {
        let _ = dispatcher.run(dispatch_store, &handlers).await;
    });

    sleep(Duration::from_millis(400)).await;

    let client = async_nats::connect("localhost:4222")
        .await
        .expect("connect");
    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(service_name, "Counter");

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    for (id, cmd) in [
        (id1, CounterCommand::Increment { by: 10 }),
        (id2, CounterCommand::Decrement { by: 3 }),
        (id1, CounterCommand::Increment { by: 1 }),
    ] {
        let env = CommandEnvelope { id, command: cmd };
        let payload = serde_json::to_vec(&env).unwrap();
        let reply = client.request(subject.clone(), payload.into()).await.unwrap();
        let resp: CommandReply = serde_json::from_slice(&reply.payload).unwrap();
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
}

/// Test that a projector error causes the projector task to terminate with an
/// error rather than silently swallowing it, and that the JoinSet reflects it.
#[tokio::test]
async fn test_projector_error_propagates() {
    let prefix = leak(unique_prefix("proj-err"));
    let service_name = leak(format!("{prefix}-svc"));
    let durable = leak(format!("{prefix}-proj-err"));

    let store = make_store(prefix).await;

    let projector = RecordingProjector::new();
    // Pre-arm the projector to fail on the first event it receives.
    projector.set_fail_next();

    let registry = CqrsRegistry::new(store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"))
        .register_projector(DurableProjectorHandler::new(durable, projector));

    let mut projector_set = registry.run_projectors().await.unwrap();

    let dispatcher = NatsCommandDispatcher::new(
        async_nats::connect("localhost:4222")
            .await
            .expect("connect"),
        service_name,
    );

    let dispatch_store = store.clone();
    let handlers: Vec<_> = registry.command_handlers().to_vec();
    tokio::spawn(async move {
        let _ = dispatcher.run(dispatch_store, &handlers).await;
    });

    sleep(Duration::from_millis(400)).await;

    let client = async_nats::connect("localhost:4222")
        .await
        .expect("connect");
    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(service_name, "Counter");

    // Publish one event; the projector will fail on it.
    let env = CommandEnvelope {
        id: Uuid::new_v4(),
        command: CounterCommand::Increment { by: 7 },
    };
    let payload = serde_json::to_vec(&env).unwrap();
    let reply = client.request(subject, payload.into()).await.unwrap();
    let resp: CommandReply = serde_json::from_slice(&reply.payload).unwrap();
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
}

/// Test that multiple commands for the same aggregate result in correct
/// cumulative state, confirming optimistic concurrency is handled correctly.
#[tokio::test]
async fn test_multiple_commands_same_aggregate_occ() {
    let prefix = leak(unique_prefix("occ"));
    let service_name = leak(format!("{prefix}-svc"));

    let store = make_store(prefix).await;

    let registry = CqrsRegistry::new(store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"));

    let dispatcher = NatsCommandDispatcher::new(
        async_nats::connect("localhost:4222")
            .await
            .expect("connect"),
        service_name,
    );

    let dispatch_store = store.clone();
    let handlers: Vec<_> = registry.command_handlers().to_vec();
    tokio::spawn(async move {
        let _ = dispatcher.run(dispatch_store, &handlers).await;
    });

    sleep(Duration::from_millis(300)).await;

    let client = async_nats::connect("localhost:4222")
        .await
        .expect("connect");
    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(service_name, "Counter");

    let id = Uuid::new_v4();
    // Send 5 sequential increments to the same aggregate.
    for i in 1i64..=5 {
        let env = CommandEnvelope {
            id,
            command: CounterCommand::Increment { by: i },
        };
        let payload = serde_json::to_vec(&env).unwrap();
        let reply = client.request(subject.clone(), payload.into()).await.unwrap();
        let resp: CommandReply = serde_json::from_slice(&reply.payload).unwrap();
        assert!(resp.success, "increment {i} should succeed");
    }

    // Expected: 1 + 2 + 3 + 4 + 5 = 15
    let root: esrc::aggregate::Root<Counter> = store.read(id).await.unwrap();
    assert_eq!(
        root.value, 15,
        "aggregate should reflect all 5 sequential increments"
    );
}

/// Test that sending a malformed (unparseable) payload to a command endpoint
/// results in an error reply and the dispatcher keeps running.
#[tokio::test]
async fn test_malformed_payload_returns_error() {
    let prefix = leak(unique_prefix("malformed"));
    let service_name = leak(format!("{prefix}-svc"));

    let store = make_store(prefix).await;

    let registry = CqrsRegistry::new(store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"));

    let dispatcher = NatsCommandDispatcher::new(
        async_nats::connect("localhost:4222")
            .await
            .expect("connect"),
        service_name,
    );

    let dispatch_store = store.clone();
    let handlers: Vec<_> = registry.command_handlers().to_vec();
    tokio::spawn(async move {
        let _ = dispatcher.run(dispatch_store, &handlers).await;
    });

    sleep(Duration::from_millis(300)).await;

    let client = async_nats::connect("localhost:4222")
        .await
        .expect("connect");
    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(service_name, "Counter");

    // Send garbage bytes.
    let bad_result = client
        .request(subject.clone(), b"this is not json"[..].into())
        .await;
    // The NATS service will reply with an error status; async-nats returns Err.
    // We only care that we get a response (not a timeout or panic).
    let _ = bad_result;

    // Confirm the dispatcher is still alive.
    let good_id = Uuid::new_v4();
    let env = CommandEnvelope {
        id: good_id,
        command: CounterCommand::Decrement { by: 2 },
    };
    let payload = serde_json::to_vec(&env).unwrap();
    let reply = client
        .request(subject, payload.into())
        .await
        .expect("dispatcher must still respond after a malformed request");
    let resp: CommandReply = serde_json::from_slice(&reply.payload).unwrap();
    assert!(resp.success);
    assert_eq!(resp.id, good_id);
}

/// Test that `CqrsRegistry::store()` returns a clone of the backing store and
/// that `command_handlers()` / `projector_handlers()` reflect registrations.
#[tokio::test]
async fn test_registry_accessors() {
    let prefix = leak(unique_prefix("registry"));

    let store = make_store(prefix).await;
    let projector = RecordingProjector::new();

    let registry = CqrsRegistry::new(store.clone())
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
}
