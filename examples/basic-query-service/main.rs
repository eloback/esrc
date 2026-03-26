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
