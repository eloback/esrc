====
>   Info: Duration: 1m 1s | ~$0.2713 | Prompt Tokens: 30,444 (cached: 0, cache_creation: 0) | Completion Tokens: 4,764 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.



Looking at the existing `multi-slice-command-service` example, I need to create a similar example that demonstrates the `QueryHandler` and `QueryClient` working with the NATS backend. Let me study the existing patterns and create a basic example.

I'll create an example that:
1. Defines a simple domain with events and an aggregate
2. Creates a `Project` implementation that maintains a read model
3. Implements a `QueryHandler` for that read model
4. Uses `QueryService` (via `spawn_query_service`) and `QueryClient` (via `get_by_id` and `query`) to validate the round-trip

<FILE_CHANGES>

<FILE_NEW file_path="examples/basic-query-service/domain.rs">
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Event};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, SerializeVersion, Event)]
pub enum OrderEvent {
    OrderPlaced { item: String, quantity: u32 },
    OrderShipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderCommand {
    PlaceOrder { item: String, quantity: u32 },
    ShipOrder,
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum OrderError {
    #[error("order already placed")]
    AlreadyPlaced,
    #[error("no order to ship")]
    NoOrder,
    #[error("order already shipped")]
    AlreadyShipped,
}

#[derive(Debug, Default)]
pub struct OrderAggregate {
    pub item: Option<String>,
    pub quantity: u32,
    pub shipped: bool,
}

impl Aggregate for OrderAggregate {
    type Command = OrderCommand;
    type Event = OrderEvent;
    type Error = OrderError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            OrderCommand::PlaceOrder { item, quantity } => {
                if self.item.is_some() {
                    Err(OrderError::AlreadyPlaced)
                } else {
                    Ok(OrderEvent::OrderPlaced { item, quantity })
                }
            },
            OrderCommand::ShipOrder => {
                if self.item.is_none() {
                    Err(OrderError::NoOrder)
                } else if self.shipped {
                    Err(OrderError::AlreadyShipped)
                } else {
                    Ok(OrderEvent::OrderShipped)
                }
            },
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            OrderEvent::OrderPlaced { item, quantity } => {
                self.item = Some(item.clone());
                self.quantity = *quantity;
            },
            OrderEvent::OrderShipped => {
                self.shipped = true;
            },
        }

        self
    }
}
</FILE_NEW>

<FILE_NEW file_path="examples/basic-query-service/read_model.rs">
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use esrc::envelope::Envelope;
use esrc::event::EventGroup;
use esrc::project::Project;
use esrc::query::{Query, QueryHandler};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::OrderEvent;

/// The read model representing a materialized order view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReadModel {
    pub id: Uuid,
    pub item: String,
    pub quantity: u32,
    pub shipped: bool,
}

/// Query enum for the order read model.
#[derive(Debug, Serialize, Deserialize)]
pub enum OrderQuery {
    /// List all orders that have been shipped.
    ListShipped,
    /// List all orders (shipped or not).
    ListAll,
}

impl Query for OrderQuery {
    type ReadModel = OrderReadModel;
    type Response = Vec<OrderReadModel>;
}

/// Shared in-memory storage used by both the projector (write side)
/// and the query handler (read side).
#[derive(Debug, Clone, Default)]
pub struct OrderStore {
    inner: Arc<Mutex<HashMap<Uuid, OrderReadModel>>>,
}

impl OrderStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: &Uuid) -> Option<OrderReadModel> {
        self.inner.lock().expect("lock not poisoned").get(id).cloned()
    }

    pub fn upsert(&self, model: OrderReadModel) {
        self.inner
            .lock()
            .expect("lock not poisoned")
            .insert(model.id, model);
    }

    pub fn all(&self) -> Vec<OrderReadModel> {
        self.inner.lock().expect("lock not poisoned").values().cloned().collect()
    }
}

/// The projector that writes into the shared OrderStore.
#[derive(Debug, Clone)]
pub struct OrderProjector {
    store: OrderStore,
}

impl OrderProjector {
    pub fn new(store: OrderStore) -> Self {
        Self { store }
    }
}

impl Project for OrderProjector {
    type EventGroup = OrderEvent;
    type Error = std::convert::Infallible;

    async fn project(&self, envelope: &Envelope<Self::EventGroup>) -> Result<(), Self::Error> {
        let id = envelope.aggregate_id();
        let event = envelope.event();

        match event {
            OrderEvent::OrderPlaced { item, quantity } => {
                self.store.upsert(OrderReadModel {
                    id,
                    item: item.clone(),
                    quantity: *quantity,
                    shipped: false,
                });
            },
            OrderEvent::OrderShipped => {
                if let Some(mut model) = self.store.get(&id) {
                    model.shipped = true;
                    self.store.upsert(model);
                }
            },
        }

        Ok(())
    }
}

/// The query handler that reads from the shared OrderStore.
#[derive(Debug)]
pub struct OrderQueryHandler {
    store: OrderStore,
}

impl OrderQueryHandler {
    pub fn new(store: OrderStore) -> Self {
        Self { store }
    }
}

impl QueryHandler for OrderQueryHandler {
    type Query = OrderQuery;
    type Id = Uuid;

    async fn get_by_id(
        &self,
        id: Self::Id,
    ) -> esrc::error::Result<Option<OrderReadModel>> {
        Ok(self.store.get(&id))
    }

    async fn handle(
        &self,
        query: Self::Query,
    ) -> esrc::error::Result<Vec<OrderReadModel>> {
        match query {
            OrderQuery::ListShipped => {
                Ok(self.store.all().into_iter().filter(|o| o.shipped).collect())
            },
            OrderQuery::ListAll => Ok(self.store.all()),
        }
    }
}
</FILE_NEW>

<FILE_NEW file_path="examples/basic-query-service/main.rs">
use std::time::Duration;

use async_nats::jetstream;
use esrc::event::{CommandClient, Truncate};
use esrc::event_modeling::{ComponentName, ReadModel};
use esrc::nats::NatsStore;
use esrc::query::{QueryClient, QuerySpec, QueryTransport};
use tokio::signal;
use uuid::Uuid;

use crate::domain::{OrderAggregate, OrderCommand, OrderEvent};
use crate::read_model::{OrderProjector, OrderQuery, OrderQueryHandler, OrderStore};

mod domain;
mod read_model;

pub const BOUNDED_CONTEXT: &str = "examples";
pub const ORDER_DOMAIN: &str = "order";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,async_nats=warn".into()),
        )
        .init();

    let client = async_nats::connect("localhost:4222").await?;
    let jetstream = jetstream::new(client);
    let store = NatsStore::try_new(jetstream, "esrc-query-example").await?;

    // Spawn the command service for the order aggregate
    store.spawn_service::<OrderAggregate>();

    // Shared in-memory store for the read model
    let order_store = OrderStore::new();

    // Spawn the read model consumer (projector writes into the shared store)
    let projector = OrderProjector::new(order_store.clone());
    let read_model_name = ComponentName::new(
        BOUNDED_CONTEXT,
        ORDER_DOMAIN,
        "order_view",
        "projector",
    );
    let read_model_spec = ReadModel::new(read_model_name, projector);
    store.spawn_read_model(read_model_spec);

    // Spawn the query service (query handler reads from the shared store)
    let query_handler = OrderQueryHandler::new(order_store.clone());
    let query_name = ComponentName::new(
        BOUNDED_CONTEXT,
        ORDER_DOMAIN,
        "order_view",
        "query",
    );
    let query_spec = QuerySpec::new(query_name.clone(), QueryTransport::NatsRequestReply, query_handler);
    store.spawn_query_service(query_spec);

    // Give the store some time to spawn services and consumers
    tokio::time::sleep(Duration::from_secs(1)).await;

    // -- Create two orders --

    let order_id_1 = Uuid::now_v7();
    let order_id_2 = Uuid::now_v7();

    // Clean up any previous state
    let _ = store.clone().truncate::<OrderEvent>(order_id_1, 0_u64.into()).await;
    let _ = store.clone().truncate::<OrderEvent>(order_id_2, 0_u64.into()).await;

    tracing::info!(order_id = %order_id_1, "placing order 1");
    store
        .send_command::<OrderAggregate>(
            order_id_1,
            OrderCommand::PlaceOrder {
                item: "Widget".to_owned(),
                quantity: 3,
            },
        )
        .await?;

    tracing::info!(order_id = %order_id_2, "placing order 2");
    store
        .send_command::<OrderAggregate>(
            order_id_2,
            OrderCommand::PlaceOrder {
                item: "Gadget".to_owned(),
                quantity: 1,
            },
        )
        .await?;

    // Ship order 1
    tracing::info!(order_id = %order_id_1, "shipping order 1");
    store
        .send_command::<OrderAggregate>(order_id_1, OrderCommand::ShipOrder)
        .await?;

    // Give the read model consumer time to process the events
    tokio::time::sleep(Duration::from_secs(2)).await;

    // -- Query via QueryClient --

    tracing::info!("--- querying via QueryClient ---");

    // get_by_id for order 1
    let result = store
        .get_by_id::<OrderQuery, _>(&query_name, order_id_1)
        .await?;
    match &result {
        Some(order) => tracing::info!(order_id = %order.id, item = %order.item, qty = order.quantity, shipped = order.shipped, "get_by_id order 1"),
        None => tracing::warn!(order_id = %order_id_1, "order 1 not found"),
    }

    // get_by_id for order 2
    let result = store
        .get_by_id::<OrderQuery, _>(&query_name, order_id_2)
        .await?;
    match &result {
        Some(order) => tracing::info!(order_id = %order.id, item = %order.item, qty = order.quantity, shipped = order.shipped, "get_by_id order 2"),
        None => tracing::warn!(order_id = %order_id_2, "order 2 not found"),
    }

    // get_by_id for a non-existent order
    let missing_id = Uuid::now_v7();
    let result = store
        .get_by_id::<OrderQuery, _>(&query_name, missing_id)
        .await?;
    match &result {
        Some(_) => tracing::warn!(order_id = %missing_id, "unexpectedly found non-existent order"),
        None => tracing::info!(order_id = %missing_id, "correctly returned None for missing order"),
    }

    // custom query: list all orders
    let all_orders = store.query::<OrderQuery>(&query_name, OrderQuery::ListAll).await?;
    tracing::info!(count = all_orders.len(), "ListAll query result");
    for order in &all_orders {
        tracing::info!(order_id = %order.id, item = %order.item, shipped = order.shipped, "  order");
    }

    // custom query: list shipped orders only
    let shipped_orders = store.query::<OrderQuery>(&query_name, OrderQuery::ListShipped).await?;
    tracing::info!(count = shipped_orders.len(), "ListShipped query result");
    for order in &shipped_orders {
        tracing::info!(order_id = %order.id, item = %order.item, shipped = order.shipped, "  shipped order");
    }

    tracing::info!("--- query validation complete, press ctrl-c to stop ---");
    signal::ctrl_c().await?;
    store.wait_graceful_shutdown().await;

    Ok(())
}
</FILE_NEW>

<FILE_PATCH file_path="_workbench/queries/plan-1-todo-steps.md">
@@
 # Plan 1 - Todo Steps
 
-## Step - In-memory QueryHandler helper for View-based live projections
+## Step - Basic query service example
+      status: active
+time-created: 2026-03-26 15:42:59
+
+- Create a basic example (`examples/basic-query-service/`) that validates the `QueryHandler` and `QueryClient` working end-to-end over NATS.
+- The example should:
+  - Define a simple order domain with events, commands, aggregate.
+  - Create a `Project` implementation that materializes an `OrderReadModel` into a shared in-memory store.
+  - Implement a `QueryHandler` that serves queries from the same shared store.
+  - Use `spawn_query_service` to expose the query handler over NATS request-reply.
+  - Use `QueryClient` (`get_by_id` and `query`) to validate the round-trip.
+- Validates: `QueryService` impl, `QueryClient` impl, `QuerySpec`, `ComponentName::query_subject()`.
+
+## Step - In-memory QueryHandler helper for View-based live projections
       status: not_started
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/queries/plan-2-active-step.md">
@@
 # Plan 2 - Active Step
 
+## Step - Basic query service example
+      status: active
+time-created: 2026-03-26 15:42:59
+time-current: 2026-03-26 15:42:59
+
+- Created `examples/basic-query-service/` with three files:
+  - `domain.rs`: defines `OrderEvent`, `OrderCommand`, `OrderError`, `OrderAggregate`.
+  - `read_model.rs`: defines `OrderReadModel`, `OrderQuery`, `OrderStore` (shared `HashMap`-based storage), `OrderProjector` (implements `Project`), `OrderQueryHandler` (implements `QueryHandler`).
+  - `main.rs`: wires everything together; spawns command service, read model consumer, and query service; places two orders; ships one; then queries via `QueryClient` to validate `get_by_id` (found, found, missing) and custom queries (`ListAll`, `ListShipped`).
+- Pattern mirrors the existing `multi-slice-command-service` example for consistency.
+- Validates the full round-trip: command -> event -> projection -> query service -> query client.
</FILE_PATCH>

</FILE_CHANGES>

Wait, I need to also check that the plan-1-todo-steps doesn't have a duplicate. Let me re-read my patch. The patch adds the active step at the top of todo, but per the rules, the step should be moved from todo to active (not remain in todo). Let me fix the plan files properly.

<FILE_CHANGES>

<FILE_PATCH file_path="_workbench/queries/plan-1-todo-steps.md">
@@
 # Plan 1 - Todo Steps
 
 ## Step - In-memory QueryHandler helper for View-based live projections
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/queries/plan-2-active-step.md">
@@
 # Plan 2 - Active Step
 
+## Step - Basic query service example
+      status: active
+time-created: 2026-03-26 15:42:59
+time-current: 2026-03-26 15:42:59
+
+- Created `examples/basic-query-service/` with three files:
+  - `domain.rs`: defines `OrderEvent`, `OrderCommand`, `OrderError`, `OrderAggregate`.
+  - `read_model.rs`: defines `OrderReadModel`, `OrderQuery`, `OrderStore` (shared `HashMap`-based storage), `OrderProjector` (implements `Project`), `OrderQueryHandler` (implements `QueryHandler`).
+  - `main.rs`: wires everything together; spawns command service, read model consumer, and query service; places two orders; ships one; then queries via `QueryClient` to validate `get_by_id` (found, found, missing) and custom queries (`ListAll`, `ListShipped`).
+- Pattern mirrors the existing `multi-slice-command-service` example for consistency.
+- Validates the full round-trip: command -> event -> projection -> query service -> query client.
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git add -A . && git commit -a -m "+ basic-query-service - Add example validating QueryHandler and QueryClient over NATS"
</suggested_git_command>

