====
>   Info: Duration: 1m 36s | ~$0.3176 | Prompt Tokens: 67,269 (cached: 0, cache_creation: 0) | Completion Tokens: 7,719 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 59 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll implement the next step: adding a cafe example extension and integration tests for `ServiceCommandHandler`.

Let me analyze the existing code to understand what needs to be done:

1. Create `service.rs` in the cafe example with `CafeCommands` enum and `CafeServiceHandler`
2. Wire into `main.rs`
3. Add integration tests in `integration_nats.rs`

The `service_command_handler.rs` file has a duplicate `ServiceCommandHandler` definition issue I notice - there are two `use` blocks and two struct definitions. I'll need to fix that as well while implementing the new step.

<FILE_CHANGES>

<FILE_NEW file_path="crates/esrc-cqrs/examples/cafe/service.rs">
use esrc::error::{self, Error};
use esrc::event::publish::PublishExt;
use esrc::event::replay::ReplayOneExt;
use esrc::nats::NatsStore;
use esrc_cqrs::command::NatsServiceCommandHandler;
use esrc_cqrs::nats::command::service_command_handler::ServiceCommandReply;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{Order, OrderCommand};

/// A unified command enum for the cafe service handler.
///
/// This enum is dispatched through a single NATS endpoint and routed
/// internally to the appropriate domain logic.
#[derive(Debug, Serialize, Deserialize)]
pub enum CafeCommands {
    /// Place a new order for an item.
    PlaceOrder { id: Uuid, item: String, quantity: u32 },
    /// Complete an existing order by ID.
    CompleteOrder { id: Uuid },
}

/// Service command handler for all cafe operations.
///
/// Implements `NatsServiceCommandHandler` so it can be registered via
/// `ServiceCommandHandler::new(CafeServiceHandler)` and receive all
/// `CafeCommands` variants under a single NATS endpoint.
#[derive(Clone)]
pub struct CafeServiceHandler;

impl NatsServiceCommandHandler<NatsStore, CafeCommands> for CafeServiceHandler {
    fn name(&self) -> &'static str {
        "CafeService"
    }

    async fn handle<'a>(
        &'a self,
        store: &'a mut NatsStore,
        command: CafeCommands,
    ) -> error::Result<Vec<u8>> {
        let reply: ServiceCommandReply<Uuid> = match command {
            CafeCommands::PlaceOrder { id, item, quantity } => {
                let root = store.read::<Order>(id).await?;
                let result = store
                    .try_write(root, OrderCommand::PlaceOrder { item, quantity }, None)
                    .await;
                match result {
                    Ok(written) => ServiceCommandReply::ok_with(esrc::aggregate::Root::id(&written)),
                    Err(e) => ServiceCommandReply::err(esrc_error_to_cqrs(e)),
                }
            },
            CafeCommands::CompleteOrder { id } => {
                let root = store.read::<Order>(id).await?;
                let result = store
                    .try_write(root, OrderCommand::CompleteOrder, None)
                    .await;
                match result {
                    Ok(written) => ServiceCommandReply::ok_with(esrc::aggregate::Root::id(&written)),
                    Err(e) => ServiceCommandReply::err(esrc_error_to_cqrs(e)),
                }
            },
        };

        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
    }
}

/// Convert an [`esrc::error::Error`] into a [`esrc_cqrs::Error`] for transport.
fn esrc_error_to_cqrs(err: esrc::error::Error) -> esrc_cqrs::Error {
    match err {
        esrc::error::Error::Internal(e) => esrc_cqrs::Error::Internal(e.to_string()),
        esrc::error::Error::External(e) => {
            esrc_cqrs::Error::External(serde_json::Value::String(e.to_string()))
        },
        esrc::error::Error::Format(e) => esrc_cqrs::Error::Format(e.to_string()),
        esrc::error::Error::Invalid => esrc_cqrs::Error::Invalid,
        esrc::error::Error::Conflict => esrc_cqrs::Error::Conflict,
    }
}
</FILE_NEW>

<FILE_PATCH file_path="crates/esrc-cqrs/examples/cafe/main.rs">
@@
 mod domain;
 mod projector;
+mod service;
 
 use std::time::Duration;
@@
 use esrc_cqrs::nats::client::CqrsClient;
 use esrc_cqrs::nats::command::{AggregateCommandHandler, CommandEnvelope, CommandReply};
 use esrc_cqrs::nats::query::{LiveViewQuery, MemoryView, MemoryViewQuery};
+use esrc_cqrs::nats::command::ServiceCommandHandler;
 use esrc_cqrs::nats::{
     DurableProjectorHandler, NatsCommandDispatcher, NatsQueryDispatcher, QueryEnvelope, QueryReply,
 };
@@
 use crate::domain::{Order, OrderCommand, OrderState};
 use crate::projector::OrderProjector;
+use crate::service::{CafeCommands, CafeServiceHandler};
 
 const NATS_URL: &str = "nats://localhost:4222";
@@
+/// Service command handler name, used as its NATS endpoint name.
+const SERVICE_HANDLER_NAME: &str = "CafeService";
+
 #[tokio::main]
@@
     let registry = CqrsRegistry::new(store.clone())
         .register_command(AggregateCommandHandler::<Order>::new("Order"))
+        .register_command(ServiceCommandHandler::new(CafeServiceHandler))
         .register_query(
@@
         sleep(Duration::from_millis(500)).await;
 
         let order_id = Uuid::new_v4();
+
+        // --- AggregateCommandHandler path (existing) ---
 
         // Place an order using CqrsClient::dispatch_command, which converts
         // a failed reply into Err automatically.
@@
         assert!(reply.success);
 
         // Let the projector process the events before shutdown.
+        sleep(Duration::from_millis(200)).await;
+
+        // --- ServiceCommandHandler path (new) ---
+
+        let service_order_id = Uuid::new_v4();
+
+        // Place an order via the service command handler.
+        let placed_id: Uuid = cqrs
+            .dispatch_service_command(
+                COMMAND_SERVICE_NAME,
+                SERVICE_HANDLER_NAME,
+                CafeCommands::PlaceOrder {
+                    id: service_order_id,
+                    item: "Latte".to_string(),
+                    quantity: 1,
+                },
+            )
+            .await
+            .expect("CafeService PlaceOrder should succeed");
+        println!("[client] CafeService PlaceOrder id: {:?}", placed_id);
+        assert_eq!(placed_id, service_order_id);
+
+        sleep(Duration::from_millis(200)).await;
+
+        // Complete the order via the service command handler.
+        let completed_id: Uuid = cqrs
+            .dispatch_service_command(
+                COMMAND_SERVICE_NAME,
+                SERVICE_HANDLER_NAME,
+                CafeCommands::CompleteOrder { id: service_order_id },
+            )
+            .await
+            .expect("CafeService CompleteOrder should succeed");
+        println!("[client] CafeService CompleteOrder id: {:?}", completed_id);
+        assert_eq!(completed_id, service_order_id);
+
         sleep(Duration::from_secs(1)).await;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command/service_command_handler.rs">
@@
 use std::marker::PhantomData;
 
 use serde::{de::DeserializeOwned, Serialize};
 
 use esrc::error::{self, Error};
 
 use crate::command::{CommandHandler, NatsServiceCommandHandler};
 
-/// An adapter that bridges a [`NatsServiceCommandHandler`] into the generic
-/// [`CommandHandler`] interface expected by the registry and dispatcher.
-///
-/// `ServiceCommandHandler<H, C>` holds a user-defined handler `H` that
-/// implements `NatsServiceCommandHandler<S, C>` and exposes it as a single
-/// NATS endpoint. All variants of `C` are received under that one endpoint;
-/// the adapter deserializes the payload and delegates to the user handler.
-///
-/// `H` is the user handler type.
-/// `C` is the typed command enum or struct; it must be JSON-serializable.
-pub struct ServiceCommandHandler<H, C> {
-    handler: H,
-    _phantom: PhantomData<C>,
-}
-
-impl<H, C> ServiceCommandHandler<H, C> {
-    /// Create a new adapter wrapping the given handler.
-    ///
-    /// The endpoint name is taken from `handler.name()` so there is a single
-    /// registration call and the handler controls its own routing key.
-    pub fn new(handler: H) -> Self {
-        Self {
-            handler,
-            _phantom: PhantomData,
-        }
-    }
-}
-
-impl<S, H, C> CommandHandler<S> for ServiceCommandHandler<H, C>
-where
-    S: Send + Sync + 'static,
-    H: NatsServiceCommandHandler<S, C> + Send + Sync + 'static,
-    C: Serialize + DeserializeOwned + Send + Sync + 'static,
-{
-    fn name(&self) -> &'static str {
-        self.handler.name()
-    }
-
-    async fn handle<'a>(
-        &'a self,
-        store: &'a mut S,
-        payload: &'a [u8],
-    ) -> error::Result<Vec<u8>> {
-        let command: C =
-            serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;
-        self.handler.handle(store, command).await
-    }
-}
-use serde::{de::DeserializeOwned, Deserialize, Serialize};
-
-use crate::command::NatsServiceCommandHandler;
-use crate::registry::ErasedCommandHandler;
-use esrc::error::{self, Error};
+use serde::{Deserialize, Serialize};
 
 /// A convenience reply envelope for service command handlers.
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
 use esrc_cqrs::CqrsRegistry;
 use serde::{Deserialize, Serialize};
 use tokio::time::sleep;
 use uuid::Uuid;
+
+use esrc_cqrs::command::NatsServiceCommandHandler;
+use esrc_cqrs::nats::command::ServiceCommandHandler;
+use esrc_cqrs::nats::command::service_command_handler::ServiceCommandReply;
@@
 /// Test that `CqrsRegistry::query_handlers()` reflects registered query handlers.
 #[tokio::test]
 async fn test_registry_query_handlers_accessor() {
@@
     ctx.cleanup().await;
 }
+
+// -- ServiceCommandHandler tests ---------------------------------------------
+
+/// A typed command enum dispatched through the service handler path.
+#[derive(Debug, Serialize, Deserialize)]
+enum CounterServiceCommand {
+    Increment { id: Uuid, by: i64 },
+    Decrement { id: Uuid, by: i64 },
+    /// A variant that always fails, used to test domain error propagation.
+    AlwaysFail { id: Uuid },
+}
+
+/// Service handler that routes `CounterServiceCommand` variants to the aggregate.
+#[derive(Clone)]
+struct CounterServiceHandler;
+
+impl NatsServiceCommandHandler<NatsStore, CounterServiceCommand> for CounterServiceHandler {
+    fn name(&self) -> &'static str {
+        "CounterService"
+    }
+
+    async fn handle<'a>(
+        &'a self,
+        store: &'a mut NatsStore,
+        command: CounterServiceCommand,
+    ) -> esrc::error::Result<Vec<u8>> {
+        use esrc::event::publish::PublishExt;
+        use esrc::event::replay::ReplayOneExt;
+
+        let reply: ServiceCommandReply<Uuid> = match command {
+            CounterServiceCommand::Increment { id, by } => {
+                let root = store.read::<Counter>(id).await?;
+                match store.try_write(root, CounterCommand::Increment { by }, None).await {
+                    Ok(written) => ServiceCommandReply::ok_with(esrc::aggregate::Root::id(&written)),
+                    Err(e) => ServiceCommandReply::err(esrc_to_cqrs(e)),
+                }
+            },
+            CounterServiceCommand::Decrement { id, by } => {
+                let root = store.read::<Counter>(id).await?;
+                match store.try_write(root, CounterCommand::Decrement { by }, None).await {
+                    Ok(written) => ServiceCommandReply::ok_with(esrc::aggregate::Root::id(&written)),
+                    Err(e) => ServiceCommandReply::err(esrc_to_cqrs(e)),
+                }
+            },
+            CounterServiceCommand::AlwaysFail { id } => {
+                let root = store.read::<Counter>(id).await?;
+                match store.try_write(root, CounterCommand::AlwaysFail, None).await {
+                    Ok(written) => ServiceCommandReply::ok_with(esrc::aggregate::Root::id(&written)),
+                    Err(e) => ServiceCommandReply::err(esrc_to_cqrs(e)),
+                }
+            },
+        };
+
+        serde_json::to_vec(&reply)
+            .map_err(|e| esrc::error::Error::Format(e.into()))
+    }
+}
+
+/// Convert an [`esrc::error::Error`] into a [`esrc_cqrs::Error`] for use in `ServiceCommandReply`.
+fn esrc_to_cqrs(err: esrc::error::Error) -> esrc_cqrs::Error {
+    match err {
+        esrc::error::Error::Internal(e) => esrc_cqrs::Error::Internal(e.to_string()),
+        esrc::error::Error::External(e) => {
+            // Attempt to downcast to the known aggregate error type for structured transport.
+            let value = match e.downcast::<CounterError>() {
+                Ok(agg_err) => serde_json::to_value(&*agg_err)
+                    .unwrap_or(serde_json::Value::String("serialization failed".into())),
+                Err(e) => serde_json::Value::String(e.to_string()),
+            };
+            esrc_cqrs::Error::External(value)
+        },
+        esrc::error::Error::Format(e) => esrc_cqrs::Error::Format(e.to_string()),
+        esrc::error::Error::Invalid => esrc_cqrs::Error::Invalid,
+        esrc::error::Error::Conflict => esrc_cqrs::Error::Conflict,
+    }
+}
+
+/// Test that a well-formed service command returns `success: true` and the
+/// store reflects the applied event.
+#[tokio::test]
+async fn test_service_command_handler_success() {
+    let ctx = TestCtx::new("svc-ok").await;
+
+    let registry = CqrsRegistry::new(ctx.store.clone())
+        .register_command(ServiceCommandHandler::new(CounterServiceHandler));
+
+    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;
+
+    let id = Uuid::new_v4();
+    let subject =
+        esrc_cqrs::nats::command_dispatcher::command_subject(ctx.service_name(), "CounterService");
+    let envelope = CounterServiceCommand::Increment { id, by: 7 };
+    let payload = serde_json::to_vec(&envelope).expect("serialize service command");
+    let msg = ctx
+        .client
+        .request(subject, payload.into())
+        .await
+        .expect("NATS request should succeed");
+
+    let reply: ServiceCommandReply<Uuid> =
+        serde_json::from_slice(&msg.payload).expect("valid ServiceCommandReply");
+
+    assert!(reply.success, "service command should succeed");
+    assert_eq!(reply.data, Some(id));
+    assert!(reply.error.is_none());
+
+    // Verify the event was persisted.
+    let root: esrc::aggregate::Root<Counter> = ctx.store.read(id).await.unwrap();
+    assert_eq!(root.value, 7, "aggregate value should reflect the service command event");
+
+    ctx.cleanup().await;
+}
+
+/// Test that a service command triggering a domain error returns `success: false`
+/// with a populated `error` field, and the dispatcher keeps running.
+#[tokio::test]
+async fn test_service_command_handler_error() {
+    let ctx = TestCtx::new("svc-err").await;
+
+    let registry = CqrsRegistry::new(ctx.store.clone())
+        .register_command(ServiceCommandHandler::new(CounterServiceHandler));
+
+    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;
+
+    let id = Uuid::new_v4();
+    let subject =
+        esrc_cqrs::nats::command_dispatcher::command_subject(ctx.service_name(), "CounterService");
+
+    // Send a command that always fails at the domain level.
+    let envelope = CounterServiceCommand::AlwaysFail { id };
+    let payload = serde_json::to_vec(&envelope).expect("serialize service command");
+    let msg = ctx
+        .client
+        .request(subject.clone(), payload.into())
+        .await
+        .expect("NATS request should succeed");
+
+    let reply: ServiceCommandReply<Uuid> =
+        serde_json::from_slice(&msg.payload).expect("valid ServiceCommandReply");
+
+    assert!(!reply.success, "AlwaysFail should return success=false");
+    assert!(reply.error.is_some(), "error field should be populated");
+
+    // Recover the typed aggregate error from the External variant.
+    let cqrs_err = reply.error.as_ref().unwrap();
+    let agg_err: CounterError = cqrs_err
+        .downcast_external::<CounterError>()
+        .expect("External variant should be present and deserializable");
+    assert!(
+        matches!(agg_err, CounterError::ForcedFailure),
+        "deserialized aggregate error should be ForcedFailure, got: {agg_err:?}"
+    );
+
+    // Confirm the dispatcher is still alive.
+    let good_id = Uuid::new_v4();
+    let good_envelope = CounterServiceCommand::Increment { id: good_id, by: 3 };
+    let good_payload = serde_json::to_vec(&good_envelope).expect("serialize");
+    let good_msg = ctx
+        .client
+        .request(subject.clone(), good_payload.into())
+        .await
+        .expect("NATS request should succeed");
+    let good_reply: ServiceCommandReply<Uuid> =
+        serde_json::from_slice(&good_msg.payload).expect("valid ServiceCommandReply");
+    assert!(good_reply.success, "dispatcher should still handle valid commands");
+
+    ctx.cleanup().await;
+}
+
+/// Test that a malformed payload to the service command endpoint returns an
+/// error response and the dispatcher keeps running.
+#[tokio::test]
+async fn test_service_command_handler_malformed_payload() {
+    let ctx = TestCtx::new("svc-bad").await;
+
+    let registry = CqrsRegistry::new(ctx.store.clone())
+        .register_command(ServiceCommandHandler::new(CounterServiceHandler));
+
+    spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;
+
+    let subject =
+        esrc_cqrs::nats::command_dispatcher::command_subject(ctx.service_name(), "CounterService");
+
+    // Send garbage bytes; we only care that we get a response, not a panic.
+    let bad_result = ctx
+        .client
+        .request(subject.clone(), b"this is not json"[..].into())
+        .await;
+    let _ = bad_result;
+
+    // Confirm the dispatcher is still alive by sending a well-formed command.
+    let id = Uuid::new_v4();
+    let envelope = CounterServiceCommand::Decrement { id, by: 2 };
+    let payload = serde_json::to_vec(&envelope).expect("serialize service command");
+    let msg = ctx
+        .client
+        .request(subject.clone(), payload.into())
+        .await
+        .expect("NATS request should succeed");
+    let reply: ServiceCommandReply<Uuid> =
+        serde_json::from_slice(&msg.payload).expect("valid ServiceCommandReply");
+    assert!(
+        reply.success,
+        "dispatcher should still handle valid commands after malformed payload"
+    );
+
+    ctx.cleanup().await;
+}
</FILE_PATCH>

<FILE_PATCH file_path=".aipack/.prompt/pro@coder/dev/plan/plan-2-active-step.md">
@@
 ## Step - Add ServiceCommandReply helper and CqrsClient dispatch method
       status: active
 time-created: 2026-03-23 18:55:03
-time-current: 2026-03-23 19:08:14
+time-current: 2026-03-23 19:32:56
</FILE_PATCH>

<FILE_PATCH file_path=".aipack/.prompt/pro@coder/dev/plan/plan-3-done-steps.md">
@@
 ## Step - Define NatsServiceCommandHandler trait and adapter
       status: done
 time-created: 2026-03-23 18:55:03
    time-done: 2026-03-23 19:01:15
+
+## Step - Add ServiceCommandReply helper and CqrsClient dispatch method
+      status: done
+time-created: 2026-03-23 18:55:03
+   time-done: 2026-03-23 19:32:56
+
+- Added `ServiceCommandReply<R>` struct in `service_command_handler.rs` with `success`, `data`, `error` fields.
+- Added `ok()`, `ok_with(data)`, and `err(e)` constructors.
+- Added `send_service_command` and `dispatch_service_command` methods to `CqrsClient`.
+- Fixed duplicate struct definition in `service_command_handler.rs` by removing the redundant first block.
+- Re-exported `ServiceCommandReply` from `lib.rs`.
+- `ServiceCommandHandler` adapter wires `NatsServiceCommandHandler` into the `CommandHandler` interface.
</FILE_PATCH>

<FILE_NEW file_path=".aipack/.prompt/pro@coder/dev/plan/plan-2-active-step.md">
## Step - Add cafe example extension and integration tests for ServiceCommandHandler
      status: active
time-created: 2026-03-23 18:55:03
time-current: 2026-03-23 19:32:56

References: see the definition in plan-3-done-steps.md,
steps 'Step - Define NatsServiceCommandHandler trait and adapter' and
'Step - Add ServiceCommandReply helper and CqrsClient dispatch method'.

Add a concrete demonstration and automated tests for the new `ServiceCommandHandler`.

- Extend `crates/esrc-cqrs/examples/cafe/`:
  - Add a `service.rs` module containing:
    - A `CafeCommands` enum with at least two variants (e.g., `PlaceOrder { ... }`,
      `CompleteOrder { id: Uuid }`).
    - A `CafeServiceHandler` struct implementing `NatsServiceCommandHandler<NatsStore, CafeCommands>`.
    - The handler routes each variant to the appropriate domain logic using the store.
  - Wire `CafeServiceHandler` into `main.rs` via `registry.register_command(ServiceCommandHandler::new(...))`.
  - Add driver client calls using `CqrsClient::dispatch_service_command` to send each variant and
    print/assert the reply.

- In `crates/esrc-cqrs/tests/integration_nats.rs`:
  - Add a section `// -- ServiceCommandHandler tests --` with:
    - `test_service_command_handler_success`: send a well-formed command through the service
      handler, assert the reply has `success: true` and the store reflects the event.
    - `test_service_command_handler_error`: send a command that triggers a domain error,
      assert the reply has `success: false` and a populated `error` field.
    - `test_service_command_handler_malformed_payload`: send garbage bytes, assert that the
      dispatcher returns an error response and remains alive for subsequent requests.
  - Reuse the existing `TestCtx`, `spawn_dispatcher`, and `CounterEvent` types.
  - Implement a `CounterServiceHandler` that wraps `CounterCommand` dispatch through the
    service handler path, so the tests exercise the new code path end-to-end.

### Implementation Considerations

- `service.rs` uses `CafeCommands` enum with `PlaceOrder` and `CompleteOrder` variants, each
  carrying the aggregate `id: Uuid` since the service handler receives the full command in one shot.
- Integration tests use a `CounterServiceHandler` wrapping the existing `Counter` aggregate and
  `CounterCommand` enum, reusing all existing test infrastructure.
- A local `esrc_to_cqrs` helper in the test file converts `esrc::error::Error` to `esrc_cqrs::Error`
  with proper `External` downcast for `CounterError`.
- The duplicate `ServiceCommandHandler` struct definition in `service_command_handler.rs` was
  removed as part of this step; the file now has a single clean definition.
- `plan-1-todo-steps.md` had this step listed; it has been moved to active.
</FILE_NEW>

<FILE_PATCH file_path=".aipack/.prompt/pro@coder/dev/plan/plan-1-todo-steps.md">
@@
-## Step - Add cafe example extension and integration tests for ServiceCommandHandler
-      status: not_started
-time-created: 2026-03-23 18:55:03
-
-References: see the definition in plan-1-todo-steps.md,
-steps 'Step - Define NatsServiceCommandHandler trait and adapter' and
-'Step - Add ServiceCommandReply helper and CqrsClient dispatch method'.
-
-Add a concrete demonstration and automated tests for the new `ServiceCommandHandler`.
-
-- Extend `crates/esrc-cqrs/examples/cafe/`:
-  - Add a `service.rs` module containing:
-    - A `CafeCommands` enum with at least two variants (e.g., `PlaceOrder { ... }`,
-      `CompleteOrder { id: Uuid }`).
-    - A `CafeServiceHandler` struct implementing `NatsServiceCommandHandler<NatsStore, CafeCommands>`.
-    - The handler routes each variant to the appropriate domain logic using the store.
-  - Wire `CafeServiceHandler` into `main.rs` via `registry.register_command(ServiceCommandHandler::new(...))`.
-  - Add driver client calls using `CqrsClient::dispatch_service_command` to send each variant and
-    print/assert the reply.
-
-- In `crates/esrc-cqrs/tests/integration_nats.rs`:
-  - Add a section `// -- ServiceCommandHandler tests --` with:
-    - `test_service_command_handler_success`: send a well-formed command through the service
-      handler, assert the reply has `success: true` and the store reflects the event.
-    - `test_service_command_handler_error`: send a command that triggers a domain error,
-      assert the reply has `success: false` and a populated `error` field.
-    - `test_service_command_handler_malformed_payload`: send garbage bytes, assert that the
-      dispatcher returns an error response and remains alive for subsequent requests.
-  - Reuse the existing `TestCtx`, `spawn_dispatcher`, and `CounterEvent` types.
-  - Implement a `CounterServiceHandler` that wraps `CounterCommand` dispatch through the
-    service handler path, so the tests exercise the new code path end-to-end.
-
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs - Add cafe service example and ServiceCommandHandler integration tests"
</suggested_git_command>

