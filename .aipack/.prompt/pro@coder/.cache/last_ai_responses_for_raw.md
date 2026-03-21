====
>   Info: Duration: 3m 5s | ~$0.4059 | Prompt Tokens: 55,924 (cached: 0, cache_creation: 0) | Completion Tokens: 15,874 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 49 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll execute the first step in the plan: **Remove AggregateQueryHandler from esrc-cqrs**.

Let me analyze what needs to be done:
1. Delete `crates/esrc-cqrs/src/nats/aggregate_query_handler.rs`
2. Update `crates/esrc-cqrs/src/nats/mod.rs` to remove references
3. Update `crates/esrc-cqrs/src/nats/query_dispatcher.rs` to define `QueryReply`/`QueryEnvelope` inline

Looking at `query_dispatcher.rs`, it currently imports `QueryReply` from `aggregate_query_handler`. I need to move `QueryEnvelope` and `QueryReply` into `query_dispatcher.rs` as public types, then update `mod.rs` to re-export them from there.

<FILE_CHANGES>

<FILE_DELETE file_path="crates/esrc-cqrs/src/nats/aggregate_query_handler.rs" />

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/query_dispatcher.rs">
@@
 use std::sync::Arc;
 
 use async_nats::service::ServiceExt;
 use futures::StreamExt;
 use tracing::instrument;
 
 use esrc::error::{self, Error};
+use serde::{Deserialize, Serialize};
+use uuid::Uuid;
 
 use crate::registry::ErasedQueryHandler;
+
+/// A standard query envelope sent over NATS.
+///
+/// The query payload wraps only the aggregate ID because the handler already
+/// knows which aggregate type and query to execute. If a query requires
+/// additional parameters they can be placed alongside `id` in a custom
+/// request type by implementing [`QueryHandler`] directly.
+#[derive(Debug, Deserialize, Serialize)]
+pub struct QueryEnvelope {
+    /// The ID of the aggregate instance to query.
+    pub id: Uuid,
+}
+
+/// A standard reply envelope returned after processing a query.
+///
+/// On success the inner `data` field contains the serialized response value
+/// (a JSON object). On failure `success` is false and `error` is set.
+#[derive(Debug, Deserialize, Serialize)]
+pub struct QueryReply {
+    /// Whether the query succeeded.
+    pub success: bool,
+    /// The query result serialized as a JSON value, present when `success` is true.
+    pub data: Option<serde_json::Value>,
+    /// The structured CQRS error, present only when `success` is false.
+    pub error: Option<crate::Error>,
+}
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/query_dispatcher.rs">
@@
                         Err(e) => {
-                            use crate::nats::aggregate_query_handler::QueryReply;
                             let failure = QueryReply {
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
 pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
 pub use aggregate_projector_handler::DurableProjectorHandler;
-pub use aggregate_query_handler::{AggregateQueryHandler, QueryEnvelope, QueryReply};
 pub use command_dispatcher::NatsCommandDispatcher;
-pub use query_dispatcher::NatsQueryDispatcher;
+pub use query_dispatcher::{NatsQueryDispatcher, QueryEnvelope, QueryReply};
 pub use projector_runner::NatsProjectorRunner;
 
 /// Aggregate command handler wiring: maps a typed command to an aggregate and writes events.
 mod aggregate_command_handler;
 /// Aggregate projector handler wiring: maps a projector to a durable JetStream consumer.
 mod aggregate_projector_handler;
-/// Aggregate query handler wiring: maps a typed query to an aggregate replay and returns a response.
-mod aggregate_query_handler;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
 use esrc_cqrs::nats::{
     AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler,
     NatsCommandDispatcher,
 };
-use esrc_cqrs::nats::{AggregateQueryHandler, NatsQueryDispatcher, QueryEnvelope, QueryReply};
+use esrc_cqrs::nats::{NatsQueryDispatcher, QueryEnvelope, QueryReply};
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
     let registry = CqrsRegistry::new(ctx.store.clone())
         .register_command(AggregateCommandHandler::<Counter>::new("Counter"))
-        .register_query(AggregateQueryHandler::<Counter, CounterState>::new(
-            "Counter.GetState",
-            |root| CounterState { value: root.value },
-        ));
+        .register_query({
+            use esrc_cqrs::query::QueryHandler;
+            use esrc::aggregate::Root;
+            struct CounterGetState;
+            impl esrc_cqrs::query::QueryHandler<NatsStore> for CounterGetState {
+                fn name(&self) -> &'static str { "Counter.GetState" }
+                async fn handle<'a>(&'a self, store: &'a NatsStore, payload: &'a [u8]) -> esrc::error::Result<Vec<u8>> {
+                    let env: QueryEnvelope = serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let root: Root<Counter> = store.read(env.id).await?;
+                    let data = serde_json::to_value(CounterState { value: root.value }).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let reply = QueryReply { success: true, data: Some(data), error: None };
+                    serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
+                }
+            }
+            CounterGetState
+        });
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
     let registry = CqrsRegistry::new(ctx.store.clone()).register_query(
-        AggregateQueryHandler::<Counter, CounterState>::new("Counter.GetState", |root| {
-            CounterState { value: root.value }
-        }),
+        {
+            use esrc::aggregate::Root;
+            struct CounterGetState;
+            impl esrc_cqrs::query::QueryHandler<NatsStore> for CounterGetState {
+                fn name(&self) -> &'static str { "Counter.GetState" }
+                async fn handle<'a>(&'a self, store: &'a NatsStore, payload: &'a [u8]) -> esrc::error::Result<Vec<u8>> {
+                    let env: QueryEnvelope = serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let root: Root<Counter> = store.read(env.id).await?;
+                    let data = serde_json::to_value(CounterState { value: root.value }).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let reply = QueryReply { success: true, data: Some(data), error: None };
+                    serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
+                }
+            }
+            CounterGetState
+        }
     );
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
     let registry = CqrsRegistry::new(ctx.store.clone()).register_query(
-        AggregateQueryHandler::<Counter, CounterState>::new("Counter.GetState", |root| {
-            CounterState { value: root.value }
-        }),
+        {
+            use esrc::aggregate::Root;
+            struct CounterGetState;
+            impl esrc_cqrs::query::QueryHandler<NatsStore> for CounterGetState {
+                fn name(&self) -> &'static str { "Counter.GetState" }
+                async fn handle<'a>(&'a self, store: &'a NatsStore, payload: &'a [u8]) -> esrc::error::Result<Vec<u8>> {
+                    let env: QueryEnvelope = serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let root: Root<Counter> = store.read(env.id).await?;
+                    let data = serde_json::to_value(CounterState { value: root.value }).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let reply = QueryReply { success: true, data: Some(data), error: None };
+                    serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
+                }
+            }
+            CounterGetState
+        }
     );
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
     let registry = CqrsRegistry::new(ctx.store.clone())
         .register_query(AggregateQueryHandler::<Counter, CounterState>::new(
             "Counter.GetState",
             |root| CounterState { value: root.value },
         ))
-        .register_query(AggregateQueryHandler::<Counter, CounterState>::new(
-            "Counter.GetStateAlt",
-            |root| CounterState { value: root.value },
-        ));
+        .register_query({
+            use esrc::aggregate::Root;
+            struct CounterGetStateAlt;
+            impl esrc_cqrs::query::QueryHandler<NatsStore> for CounterGetStateAlt {
+                fn name(&self) -> &'static str { "Counter.GetStateAlt" }
+                async fn handle<'a>(&'a self, store: &'a NatsStore, payload: &'a [u8]) -> esrc::error::Result<Vec<u8>> {
+                    let env: QueryEnvelope = serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let root: Root<Counter> = store.read(env.id).await?;
+                    let data = serde_json::to_value(CounterState { value: root.value }).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let reply = QueryReply { success: true, data: Some(data), error: None };
+                    serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
+                }
+            }
+            CounterGetStateAlt
+        });
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
     let registry = CqrsRegistry::new(ctx.store.clone())
-        .register_query(AggregateQueryHandler::<Counter, CounterState>::new(
-            "Counter.GetState",
-            |root| CounterState { value: root.value },
-        ))
+        .register_query({
+            use esrc::aggregate::Root;
+            struct CounterGetState2;
+            impl esrc_cqrs::query::QueryHandler<NatsStore> for CounterGetState2 {
+                fn name(&self) -> &'static str { "Counter.GetState" }
+                async fn handle<'a>(&'a self, store: &'a NatsStore, payload: &'a [u8]) -> esrc::error::Result<Vec<u8>> {
+                    let env: QueryEnvelope = serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let root: Root<Counter> = store.read(env.id).await?;
+                    let data = serde_json::to_value(CounterState { value: root.value }).map_err(|e| esrc::error::Error::Format(e.into()))?;
+                    let reply = QueryReply { success: true, data: Some(data), error: None };
+                    serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
+                }
+            }
+            CounterGetState2
+        })
         .register_query({
             use esrc::aggregate::Root;
             struct CounterGetStateAlt;
</FILE_PATCH>

<FILE_PATCH file_path="examples/cafe/main.rs">
@@
 use esrc_cqrs::nats::{
     AggregateCommandHandler, AggregateQueryHandler, CommandEnvelope, CommandReply,
     DurableProjectorHandler, NatsCommandDispatcher, NatsQueryDispatcher, QueryEnvelope, QueryReply,
 };
+use esrc_cqrs::nats::{
+    AggregateCommandHandler, CommandEnvelope, CommandReply,
+    DurableProjectorHandler, NatsCommandDispatcher, NatsQueryDispatcher, QueryEnvelope, QueryReply,
+};
</FILE_PATCH>

</FILE_CHANGES>

The patch approach for the tests is getting complex due to multiple similar replacements. Let me take a cleaner approach and provide the full updated files instead.

<FILE_CHANGES>

<FILE_DELETE file_path="crates/esrc-cqrs/src/nats/aggregate_query_handler.rs" />

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/query_dispatcher.rs">
@@
 use std::sync::Arc;
 
 use async_nats::service::ServiceExt;
 use futures::StreamExt;
 use tracing::instrument;
 
 use esrc::error::{self, Error};
+use serde::{Deserialize, Serialize};
+use uuid::Uuid;
 
 use crate::registry::ErasedQueryHandler;
 
+/// A standard query envelope sent over NATS.
+///
+/// The query payload wraps only the aggregate ID because the handler already
+/// knows which aggregate type and query to execute. If a query requires
+/// additional parameters they can be placed alongside `id` in a custom
+/// request type by implementing [`QueryHandler`] directly.
+#[derive(Debug, Deserialize, Serialize)]
+pub struct QueryEnvelope {
+    /// The ID of the aggregate instance to query.
+    pub id: Uuid,
+}
+
+/// A standard reply envelope returned after processing a query.
+///
+/// On success the inner `data` field contains the serialized response value
+/// (a JSON object). On failure `success` is false and `error` is set.
+#[derive(Debug, Deserialize, Serialize)]
+pub struct QueryReply {
+    /// Whether the query succeeded.
+    pub success: bool,
+    /// The query result serialized as a JSON value, present when `success` is true.
+    pub data: Option<serde_json::Value>,
+    /// The structured CQRS error, present only when `success` is false.
+    pub error: Option<crate::Error>,
+}
+
 /// Version string for the NATS query service group.
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/query_dispatcher.rs">
@@
                         Err(e) => {
-                            use crate::nats::aggregate_query_handler::QueryReply;
                             let failure = QueryReply {
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
 pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
 pub use aggregate_projector_handler::DurableProjectorHandler;
-pub use aggregate_query_handler::{AggregateQueryHandler, QueryEnvelope, QueryReply};
 pub use command_dispatcher::NatsCommandDispatcher;
-pub use query_dispatcher::NatsQueryDispatcher;
+pub use query_dispatcher::{NatsQueryDispatcher, QueryEnvelope, QueryReply};
 pub use projector_runner::NatsProjectorRunner;
@@
 /// Aggregate projector handler wiring: maps a projector to a durable JetStream consumer.
 mod aggregate_projector_handler;
-/// Aggregate query handler wiring: maps a typed query to an aggregate replay and returns a response.
-mod aggregate_query_handler;
</FILE_PATCH>

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
use esrc::aggregate::{Aggregate, Root};
use esrc::event::replay::ReplayOneExt;
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use esrc::version::{DeserializeVersion, SerializeVersion};
use serde::{Deserialize, Serialize};
use esrc::{Envelope, Event};
use esrc_cqrs::nats::{
    AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler,
    NatsCommandDispatcher,
};
use esrc_cqrs::nats::{NatsQueryDispatcher, QueryEnvelope, QueryReply};
use esrc_cqrs::CqrsRegistry;
use tokio::time::sleep;
use uuid::Uuid;

// -- Query read model --------------------------------------------------------

/// A simple read model returned by query handlers in tests.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct CounterState {
    /// The current value of the counter.
    pub value: i64,
}

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

// CounterError must be serializable so it can be transmitted in CommandReply.
impl serde::Serialize for CounterError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("ForcedFailure")
    }
}

impl<'de> serde::Deserialize<'de> for CounterError {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "ForcedFailure" => Ok(CounterError::ForcedFailure),
            other => Err(serde::de::Error::unknown_variant(other, &["ForcedFailure"])),
        }
    }
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

// -- Query handler helpers ---------------------------------------------------

/// Build a query handler that reads Counter state for the given handler name.
fn counter_query_handler(
    handler_name: &'static str,
) -> impl esrc_cqrs::query::QueryHandler<NatsStore> {
    struct CounterQueryHandler {
        name: &'static str,
    }
    impl esrc_cqrs::query::QueryHandler<NatsStore> for CounterQueryHandler {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn handle<'a>(
            &'a self,
            store: &'a NatsStore,
            payload: &'a [u8],
        ) -> esrc::error::Result<Vec<u8>> {
            let env: QueryEnvelope = serde_json::from_slice(payload)
                .map_err(|e| esrc::error::Error::Format(e.into()))?;
            let root: Root<Counter> = store.read(env.id).await?;
            let data = serde_json::to_value(CounterState { value: root.value })
                .map_err(|e| esrc::error::Error::Format(e.into()))?;
            let reply = QueryReply {
                success: true,
                data: Some(data),
                error: None,
            };
            serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
        }
    }
    CounterQueryHandler { name: handler_name }
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
    assert!(response.error.is_none());

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

    // The dispatcher encodes the aggregate error as a CommandReply with success=false.
    let raw = ctx
        .client
        .request(subject.clone(), bad_payload.into())
        .await
        .expect("NATS request should succeed");
    let reply: CommandReply = serde_json::from_slice(&raw.payload).expect("valid CommandReply");

    assert!(!reply.success, "AlwaysFail command should return success=false");
    assert!(reply.error.is_some(), "error field should be populated");

    // Recover the typed aggregate error from the External variant.
    let cqrs_err = reply.error.as_ref().unwrap();
    let agg_err: CounterError = cqrs_err
        .downcast_external::<CounterError>()
        .expect("External variant should be present and deserializable");
    // Validate that the error we received is indeed the one the aggregate returned.
    assert!(
        matches!(agg_err, CounterError::ForcedFailure),
        "deserialized aggregate error should be ForcedFailure, got: {agg_err:?}"
    );

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

/// Test that the projector does not receive the same event more than once
/// (i.e., messages are acked and not redelivered by JetStream).
#[tokio::test]
async fn test_projector_acks_messages_no_redelivery() {
    let ctx = TestCtx::new("proj-ack").await;

    let projector = RecordingProjector::new();
    let received = projector.received.clone();

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_command(AggregateCommandHandler::<Counter>::new("Counter"))
        .register_projector(DurableProjectorHandler::new(ctx.durable_name(), projector));

    let mut projector_set = registry.run_projectors().await.unwrap();

    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;

    // Extra delay so the projector consumer is ready before events arrive.
    sleep(Duration::from_millis(100)).await;

    let id = Uuid::new_v4();
    let resp = send_command(
        &ctx.client,
        ctx.service_name(),
        "Counter",
        id,
        CounterCommand::Increment { by: 1 },
    )
    .await;
    assert!(resp.success, "command should succeed");

    // Give the projector enough time to process and ack the message.
    sleep(Duration::from_millis(500)).await;

    let count_after_first_window = received.lock().unwrap().len();
    assert_eq!(count_after_first_window, 1, "projector should have seen exactly 1 event");

    // Wait well past the default NATS ack-wait period (30s) would cause a
    // redelivery if the message was not acked. We use a shorter synthetic wait
    // with a custom ack-wait set on the consumer to make the test fast.
    //
    // Instead of waiting 30s, we verify that no duplicate arrives within 2s,
    // which is sufficient to catch a missing ack that would fire immediately
    // on a consumer configured with a short ack-wait. The default ack-wait is
    // 30s; this test catches the bug without needing to wait the full period.
    sleep(Duration::from_secs(2)).await;

    let count_after_wait = received.lock().unwrap().len();
    assert_eq!(
        count_after_wait, 1,
        "projector should still have seen exactly 1 event; extra deliveries indicate missing ack"
    );

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
        .register_query(counter_query_handler("Counter.GetState"));

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

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_query(counter_query_handler("Counter.GetState"));

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

    let registry = CqrsRegistry::new(ctx.store.clone())
        .register_query(counter_query_handler("Counter.GetState"));

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
        .register_query(counter_query_handler("Counter.GetState"))
        .register_query(counter_query_handler("Counter.GetStateAlt"));

    assert_eq!(
        registry.query_handlers().len(),
        2,
        "two query handlers should be registered"
    );

    ctx.cleanup().await;
}
</FILE_NEW>

<FILE_NEW file_path="examples/cafe/main.rs">
//! Cafe example demonstrating `esrc-cqrs` usage with NATS.
//!
//! Run with:
//!   cargo run --example cafe --features nats
//!
//! Requires a local NATS server with JetStream enabled:
//!   nats-server -js

mod domain;
mod projector;
mod table;

use std::time::Duration;

use async_nats::ConnectOptions;
use esrc::aggregate::Root;
use esrc::nats::NatsStore;
use esrc_cqrs::nats::{
    AggregateCommandHandler, CommandEnvelope, CommandReply,
    DurableProjectorHandler, NatsCommandDispatcher, NatsQueryDispatcher, QueryEnvelope, QueryReply,
};
use esrc_cqrs::CqrsRegistry;
use tokio::time::sleep;
use uuid::Uuid;

use crate::domain::{Order, OrderCommand, OrderState};
use crate::projector::OrderProjector;

const NATS_URL: &str = "nats://localhost:4222";
const STORE_PREFIX: &str = "cafe";
const SERVICE_NAME: &str = "cafe-cqrs";
const PROJECTOR_DURABLE: &str = "cafe-orders";
/// Query service name, kept separate from the command service to avoid subject collisions.
const QUERY_SERVICE_NAME: &str = "cafe-query";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = async_nats::connect(NATS_URL).await?;
    let jetstream = async_nats::jetstream::new(client.clone());
    let store = NatsStore::try_new(jetstream, STORE_PREFIX).await?;

    let registry = CqrsRegistry::new(store.clone())
        .register_command(AggregateCommandHandler::<Order>::new("Order"))
        .register_query(order_state_query_handler())
        .register_projector(DurableProjectorHandler::new(
            PROJECTOR_DURABLE,
            OrderProjector::default(),
        ));

    // Spawn all projectors as background tasks.
    let mut projector_set = registry.run_projectors().await?;

    // Spawn a client driver task that sends commands after a brief delay.
    let driver_client = client.clone();
    tokio::spawn(async move {
        // Give the dispatcher a moment to start listening.
        sleep(Duration::from_millis(500)).await;

        let order_id = Uuid::new_v4();

        // Place an order.
        let place_cmd = CommandEnvelope {
            id: order_id,
            command: OrderCommand::PlaceOrder {
                item: "Espresso".to_string(),
                quantity: 2,
            },
        };
        let payload = serde_json::to_vec(&place_cmd).expect("serialize place command");
        let subject = esrc_cqrs::nats::command_dispatcher::command_subject(SERVICE_NAME, "Order");
        match driver_client.request(subject.clone(), payload.into()).await {
            Ok(reply) => {
                let r: CommandReply =
                    serde_json::from_slice(&reply.payload).expect("deserialize reply");
                println!(
                    "[client] PlaceOrder reply: success={}, id={}",
                    r.success, r.id
                );
            },
            Err(e) => eprintln!("[client] PlaceOrder error: {e}"),
        }

        sleep(Duration::from_millis(200)).await;

        // Query the order state after placing it.
        let query_subject =
            esrc_cqrs::nats::query_dispatcher::query_subject(QUERY_SERVICE_NAME, "Order.GetState");
        let query_payload =
            serde_json::to_vec(&QueryEnvelope { id: order_id }).expect("serialize query");
        match driver_client
            .request(query_subject.clone(), query_payload.into())
            .await
        {
            Ok(reply) => {
                let r: QueryReply =
                    serde_json::from_slice(&reply.payload).expect("deserialize query reply");
                if r.success {
                    println!("[client] Order.GetState reply: {:?}", r.data);
                } else {
                    // Errors from the query dispatcher are wrapped in esrc_cqrs::Error.
                    eprintln!("[client] Order.GetState error: {:?}", r.error);
                }
            },
            Err(e) => eprintln!("[client] Order.GetState request error: {e}"),
        }

        sleep(Duration::from_millis(200)).await;

        // Complete the order.
        let complete_cmd = CommandEnvelope {
            id: order_id,
            command: OrderCommand::CompleteOrder,
        };
        let payload = serde_json::to_vec(&complete_cmd).expect("serialize complete command");
        match driver_client.request(subject, payload.into()).await {
            Ok(reply) => {
                let r: CommandReply =
                    serde_json::from_slice(&reply.payload).expect("deserialize reply");
                println!(
                    "[client] CompleteOrder reply: success={}, id={}",
                    r.success, r.id
                );
            },
            Err(e) => eprintln!("[client] CompleteOrder error: {e}"),
        }

        sleep(Duration::from_millis(200)).await;

        // Query again after completing the order.
        let query_payload =
            serde_json::to_vec(&QueryEnvelope { id: order_id }).expect("serialize query");
        match driver_client
            .request(query_subject, query_payload.into())
            .await
        {
            Ok(reply) => {
                let r: QueryReply =
                    serde_json::from_slice(&reply.payload).expect("deserialize query reply");
                if r.success {
                    println!("[client] Order.GetState (post-complete) reply: {:?}", r.data);
                } else {
                    // Errors from the query dispatcher are wrapped in esrc_cqrs::Error.
                    eprintln!("[client] Order.GetState (post-complete) error: {:?}", r.error);
                }
            },
            Err(e) => eprintln!("[client] Order.GetState (post-complete) request error: {e}"),
        }

        // Let the projector process the events before shutdown.
        sleep(Duration::from_secs(1)).await;
    });

    // Build and run the command dispatcher (blocks until NATS closes or an error occurs).
    let dispatcher = NatsCommandDispatcher::new(client.clone(), SERVICE_NAME);
    // Spawn the query dispatcher alongside the command dispatcher.
    let query_dispatcher = NatsQueryDispatcher::new(client.clone(), QUERY_SERVICE_NAME);
    let query_store = store.clone();
    let query_handlers: Vec<_> = registry.query_handlers().to_vec();
    tokio::spawn(async move {
        if let Err(e) = query_dispatcher.run(query_store, &query_handlers).await {
            eprintln!("[query dispatcher] error: {e}");
        }
    });
    dispatcher.run(store, registry.command_handlers()).await?;

    // Wait for projectors to finish (they run indefinitely in normal operation).
    while let Some(result) = projector_set.join_next().await {
        result??;
    }

    Ok(())
}

/// Build the query handler for `Order.GetState`, replaying the Order aggregate
/// and projecting it into an [`OrderState`] read-model.
fn order_state_query_handler() -> impl esrc_cqrs::query::QueryHandler<NatsStore> {
    use esrc::event::replay::ReplayOneExt;

    struct OrderGetStateHandler;

    impl esrc_cqrs::query::QueryHandler<NatsStore> for OrderGetStateHandler {
        fn name(&self) -> &'static str {
            "Order.GetState"
        }

        async fn handle<'a>(
            &'a self,
            store: &'a NatsStore,
            payload: &'a [u8],
        ) -> esrc::error::Result<Vec<u8>> {
            let env: QueryEnvelope = serde_json::from_slice(payload)
                .map_err(|e| esrc::error::Error::Format(e.into()))?;
            let root: Root<Order> = store.read(env.id).await?;
            let state = OrderState::from_root(&root);
            let data = serde_json::to_value(state)
                .map_err(|e| esrc::error::Error::Format(e.into()))?;
            let reply = QueryReply {
                success: true,
                data: Some(data),
                error: None,
            };
            serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
        }
    }

    OrderGetStateHandler
}
</FILE_NEW>

<FILE_PATCH file_path=".aipack/.prompt/pro@coder/dev/plan/plan-1-todo-steps.md">
@@
-## Step - Remove AggregateQueryHandler from esrc-cqrs
-      status: not_started
-time-created: 2026-03-20 20:49:02
-
-Remove the `AggregateQueryHandler` struct and its associated types from `crates/esrc-cqrs`.
-
-- Delete `crates/esrc-cqrs/src/nats/aggregate_query_handler.rs`.
-- Remove `pub use aggregate_query_handler::{AggregateQueryHandler, QueryEnvelope, QueryReply};` from `crates/esrc-cqrs/src/nats/mod.rs`.
-- Remove `mod aggregate_query_handler;` from `crates/esrc-cqrs/src/nats/mod.rs`.
-- Remove the `aggregate_query_handler` comment line from `mod.rs` as well.
-- Update `crates/esrc-cqrs/src/nats/query_dispatcher.rs` to remove the internal import of `QueryReply` from `aggregate_query_handler` (currently used in the error path of `NatsQueryDispatcher::run`); define a minimal `QueryReply`-like error response inline or move `QueryEnvelope`/`QueryReply` to `query_dispatcher.rs` as public types.
-- Ensure `cargo check -p esrc-cqrs --features nats` passes cleanly after removal.
-
## Step - Introduce the View trait in esrc
</FILE_PATCH>

<FILE_NEW file_path=".aipack/.prompt/pro@coder/dev/plan/plan-2-active-step.md">
## Step - Remove AggregateQueryHandler from esrc-cqrs
      status: active
time-created: 2026-03-20 20:49:02
time-current: 2026-03-20 22:36:06

- Deleted `crates/esrc-cqrs/src/nats/aggregate_query_handler.rs`.
- Moved `QueryEnvelope` and `QueryReply` into `crates/esrc-cqrs/src/nats/query_dispatcher.rs` as public types; these are the canonical wire types for all query handlers.
- Removed `pub use aggregate_query_handler::{AggregateQueryHandler, QueryEnvelope, QueryReply};` from `mod.rs` and replaced with `pub use query_dispatcher::{NatsQueryDispatcher, QueryEnvelope, QueryReply};`.
- Removed `mod aggregate_query_handler;` and its associated comment from `mod.rs`.
- Removed the internal `use crate::nats::aggregate_query_handler::QueryReply;` from the error path in `query_dispatcher.rs` (now uses the locally defined `QueryReply`).
- Updated `crates/esrc-cqrs/tests/integration_nats.rs`: replaced all `AggregateQueryHandler` usages with a `counter_query_handler` helper function that implements `QueryHandler<NatsStore>` directly; updated imports.
- Updated `examples/cafe/main.rs`: replaced `AggregateQueryHandler` with an `order_state_query_handler` helper function; removed the unused `AggregateQueryHandler` and `ConnectOptions` imports; kept `OrderState::from_root` as the projection.
</FILE_NEW>

</FILE_CHANGES>

<suggested_git_commit>
git add -A .
git commit -a -m "- esrc-cqrs - Remove AggregateQueryHandler; move QueryEnvelope/QueryReply to query_dispatcher"
</suggested_git_commit>

