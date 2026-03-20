====
>   Info: Duration: 38s | ~$0.0878 | Prompt Tokens: 14,864 (cached: 0, cache_creation: 0) | Completion Tokens: 2,881 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the existing code, I need to:

1. Update `examples/cafe/domain.rs` to add a query response type (`OrderState`).
2. Update `examples/cafe/projector.rs` - no changes needed there.
3. Add a new `examples/cafe/query.rs` with an `OrderQueryHandler` (or use `AggregateQueryHandler`).
4. Update `examples/cafe/main.rs` to register a query handler and demonstrate sending a query, using the `NatsQueryDispatcher` and wrapping errors with `esrc_cqrs::Error`.

I also need to check that the `NatsQueryDispatcher` is used properly and that errors are wrapped in `esrc_cqrs::Error` as noted.

Looking at `main.rs`, it currently only shows commands. I need to:
- Add a query to fetch order state after placing it.
- Register a `AggregateQueryHandler` for `Order`.
- Run a `NatsQueryDispatcher` alongside the command dispatcher.
- Show that query errors come back as `esrc_cqrs::Error`.

I also need to add an `OrderState` read model to `domain.rs` and derive the right traits.

Let me also check what needs to be added to `Cargo.toml` for the example - it already has `esrc-cqrs` as a dev-dependency, so that's fine.

<FILE_CHANGES>

<FILE_PATCH file_path="examples/cafe/domain.rs">
@@
 use esrc::aggregate::Aggregate;
+use esrc::aggregate::Root;
 use esrc::version::{DeserializeVersion, SerializeVersion};
@@
 /// Errors that can occur when processing Order commands.
 #[derive(Debug, Serialize, Deserialize, thiserror::Error)]
 pub enum OrderError {
     #[error("order is already completed")]
     AlreadyCompleted,
     #[error("order has not been placed yet")]
     NotPlaced,
 }
+
+/// A read-model snapshot of an Order aggregate, returned by queries.
+#[derive(Debug, Clone, Serialize, Deserialize)]
+pub struct OrderState {
+    /// The current status of the order.
+    pub status: String,
+    /// The item that was ordered, if any.
+    pub item: Option<String>,
+    /// The quantity ordered.
+    pub quantity: u32,
+}
+
+impl OrderState {
+    /// Project an [`Order`] aggregate root into an [`OrderState`] read-model.
+    pub fn from_root(root: &Root<Order>) -> Self {
+        let order = Root::state(root);
+        Self {
+            status: format!("{:?}", order.status),
+            item: order.item.clone(),
+            quantity: order.quantity,
+        }
+    }
+}
</FILE_PATCH>

<FILE_PATCH file_path="examples/cafe/main.rs">
@@
 //! Cafe example demonstrating `esrc-cqrs` usage with NATS.
 //!
 //! Run with:
 //!   cargo run --example cafe --features nats
 //!
 //! Requires a local NATS server with JetStream enabled:
 //!   nats-server -js

 mod domain;
 mod projector;
+mod table;

 use std::time::Duration;

 use async_nats::ConnectOptions;
 use esrc::nats::NatsStore;
 use esrc_cqrs::nats::{
-    AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler,
-    NatsCommandDispatcher,
+    AggregateCommandHandler, AggregateQueryHandler, CommandEnvelope, CommandReply,
+    DurableProjectorHandler, NatsCommandDispatcher, NatsQueryDispatcher, QueryEnvelope, QueryReply,
 };
 use esrc_cqrs::CqrsRegistry;
 use tokio::time::sleep;
 use uuid::Uuid;

-use crate::domain::{Order, OrderCommand};
+use crate::domain::{Order, OrderCommand, OrderState};
 use crate::projector::OrderProjector;

 const NATS_URL: &str = "nats://localhost:4222";
 const STORE_PREFIX: &str = "cafe";
 const SERVICE_NAME: &str = "cafe-cqrs";
 const PROJECTOR_DURABLE: &str = "cafe-orders";
+/// Query service name, kept separate from the command service to avoid subject collisions.
+const QUERY_SERVICE_NAME: &str = "cafe-query";

 #[tokio::main]
 async fn main() -> Result<(), Box<dyn std::error::Error>> {
     let client = async_nats::connect(NATS_URL).await?;
     let jetstream = async_nats::jetstream::new(client.clone());
     let store = NatsStore::try_new(jetstream, STORE_PREFIX).await?;

     let registry = CqrsRegistry::new(store.clone())
         .register_command(AggregateCommandHandler::<Order>::new("Order"))
+        .register_query(AggregateQueryHandler::<Order, OrderState>::new(
+            "Order.GetState",
+            OrderState::from_root,
+        ))
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

+        // Query the order state after placing it.
+        let query_subject =
+            esrc_cqrs::nats::query_dispatcher::query_subject(QUERY_SERVICE_NAME, "Order.GetState");
+        let query_payload =
+            serde_json::to_vec(&QueryEnvelope { id: order_id }).expect("serialize query");
+        match driver_client
+            .request(query_subject.clone(), query_payload.into())
+            .await
+        {
+            Ok(reply) => {
+                let r: QueryReply =
+                    serde_json::from_slice(&reply.payload).expect("deserialize query reply");
+                if r.success {
+                    println!("[client] Order.GetState reply: {:?}", r.data);
+                } else {
+                    // Errors from the query dispatcher are wrapped in esrc_cqrs::Error.
+                    eprintln!("[client] Order.GetState error: {:?}", r.error);
+                }
+            },
+            Err(e) => eprintln!("[client] Order.GetState request error: {e}"),
+        }
+
+        sleep(Duration::from_millis(200)).await;
+
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

+        sleep(Duration::from_millis(200)).await;
+
+        // Query again after completing the order.
+        let query_payload =
+            serde_json::to_vec(&QueryEnvelope { id: order_id }).expect("serialize query");
+        match driver_client
+            .request(query_subject, query_payload.into())
+            .await
+        {
+            Ok(reply) => {
+                let r: QueryReply =
+                    serde_json::from_slice(&reply.payload).expect("deserialize query reply");
+                if r.success {
+                    println!("[client] Order.GetState (post-complete) reply: {:?}", r.data);
+                } else {
+                    // Errors from the query dispatcher are wrapped in esrc_cqrs::Error.
+                    eprintln!("[client] Order.GetState (post-complete) error: {:?}", r.error);
+                }
+            },
+            Err(e) => eprintln!("[client] Order.GetState (post-complete) request error: {e}"),
+        }
+
         // Let the projector process the events before shutdown.
         sleep(Duration::from_secs(1)).await;
     });

     // Build and run the command dispatcher (blocks until NATS closes or an error occurs).
     let dispatcher = NatsCommandDispatcher::new(client.clone(), SERVICE_NAME);
-    dispatcher.run(store, registry.command_handlers()).await?;
+    // Spawn the query dispatcher alongside the command dispatcher.
+    let query_dispatcher = NatsQueryDispatcher::new(client.clone(), QUERY_SERVICE_NAME);
+    let query_store = store.clone();
+    let query_handlers: Vec<_> = registry.query_handlers().to_vec();
+    tokio::spawn(async move {
+        if let Err(e) = query_dispatcher.run(query_store, &query_handlers).await {
+            eprintln!("[query dispatcher] error: {e}");
+        }
+    });
+    dispatcher.run(store, registry.command_handlers()).await?;

     // Wait for projectors to finish (they run indefinitely in normal operation).
     while let Some(result) = projector_set.join_next().await {
         result??;
     }

     Ok(())
 }
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git add -A .
git commit -a -m "^ cafe example - Add query handler registration and query dispatch with esrc_cqrs::Error"
</suggested_git_command>

