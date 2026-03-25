//! Cafe example demonstrating `esrc` command service usage with NATS.
//!
//! Run with:
//!   cargo run --example cafe --features nats
//!
//! Requires a local NATS server with JetStream enabled:
//!   nats-server -js

mod domain;

use std::time::Duration;

use esrc::event::command_service::CommandError;
use esrc::event::replay::ReplayOneExt;
use esrc::nats::NatsStore;
use tokio::time::sleep;
use uuid::Uuid;

use crate::domain::{Order, OrderCommand};

const NATS_URL: &str = "nats://localhost:4222";
const STORE_PREFIX: &str = "cafe";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = async_nats::connect(NATS_URL).await?;
    let jetstream = async_nats::jetstream::new(client.clone());
    let store = NatsStore::try_new(jetstream, STORE_PREFIX).await?;

    // Spawn the aggregate command service as a background task.
    store.spawn_service::<Order>();

    // Spawn a client driver task that sends commands after a brief delay.
    let driver_client = client.clone();
    let driver_store = store.clone();
    tokio::spawn(async move {
        // Give the command service a moment to start listening.
        sleep(Duration::from_millis(500)).await;

        let order_id = Uuid::now_v7();
        let event_name = <domain::OrderEvent as esrc::event::Event>::name();

        // Place an order via the command service endpoint.
        let place_cmd = OrderCommand::PlaceOrder {
            item: "Espresso".to_string(),
            quantity: 2,
        };
        let payload = serde_json::to_vec(&place_cmd).expect("serialize PlaceOrder");
        let subject = format!("{event_name}.command.{order_id}");
        let reply = driver_client
            .request(subject.clone(), payload.into())
            .await
            .expect("PlaceOrder request failed");

        if reply.payload.is_empty() {
            println!("[client] PlaceOrder succeeded for {order_id}");
        } else {
            let err: CommandError =
                serde_json::from_slice(&reply.payload).expect("deserialize error");
            panic!("[client] PlaceOrder failed: {err}");
        }

        sleep(Duration::from_millis(200)).await;

        // Query the aggregate state via direct replay.
        let root = driver_store
            .read::<Order>(order_id)
            .await
            .expect("read aggregate");
        let order = esrc::aggregate::Root::into_inner(root);
        println!(
            "[client] Order state after PlaceOrder: status={:?}, item={:?}, quantity={}",
            order.status, order.item, order.quantity
        );

        // Complete the order via the command service endpoint.
        let complete_cmd = OrderCommand::CompleteOrder;
        let payload = serde_json::to_vec(&complete_cmd).expect("serialize CompleteOrder");
        let subject = format!("{event_name}.command.{order_id}");
        let reply = driver_client
            .request(subject, payload.into())
            .await
            .expect("CompleteOrder request failed");

        if reply.payload.is_empty() {
            println!("[client] CompleteOrder succeeded for {order_id}");
        } else {
            let err: CommandError =
                serde_json::from_slice(&reply.payload).expect("deserialize error");
            panic!("[client] CompleteOrder failed: {err}");
        }

        sleep(Duration::from_millis(200)).await;

        // Query the aggregate state again.
        let root = driver_store
            .read::<Order>(order_id)
            .await
            .expect("read aggregate");
        let order = esrc::aggregate::Root::into_inner(root);
        println!(
            "[client] Order state after CompleteOrder: status={:?}, item={:?}, quantity={}",
            order.status, order.item, order.quantity
        );

        sleep(Duration::from_secs(1)).await;
    });

    // Block on the command service (runs until NATS closes or shutdown).
    store.wait_graceful_shutdown().await;

    Ok(())
}
