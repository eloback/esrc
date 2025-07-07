#![allow(clippy::unnecessary_literal_unwrap)]
#![allow(unused)]

use async_nats::jetstream;
use esrc::aggregate::Root;
use esrc::event::{PublishExt, ReplayExt, ReplayOneExt, SubscribeExt};
use esrc::nats::NatsStore;
use std::time::Duration;
use tab::{Item, Tab, TabCommand};
use table::ActiveTables;
use tokio::time::sleep;
use uuid::Uuid;

mod error;
mod tab;
mod table;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = async_nats::connect("localhost").await?;
    let context = jetstream::new(client);

    let store = NatsStore::try_new(context, "cafe").await?;

    let active_tables = ActiveTables::new();

    // Start observing in a background task
    // tokio::spawn({
    //     let store_clone = store.clone();
    //     async move {
    //         if let Err(e) = store_clone.observe(active_tables).await {
    //             eprintln!("Observer error: {}", e);
    //         }
    //     }
    // });

    // Give the observer time to start
    sleep(Duration::from_millis(100)).await;

    // Demonstrate the system by creating some tabs
    println!("Starting cafe example...");

    let table_42_id = on_open(store.clone(), 42, "Derek".to_string()).await?;
    println!("Opened tab for table 42 with ID: {}", table_42_id);

    let table_15_id = on_open(store.clone(), 15, "Alice".to_string()).await?;
    println!("Opened tab for table 15 with ID: {}", table_15_id);

    // Wait a bit for the events to be processed
    sleep(Duration::from_millis(500)).await;

    // Check if tables are active
    let is_42_active = is_open(store.clone(), 42).await?;
    let is_15_active = is_open(store.clone(), 15).await?;
    let is_99_active = is_open(store.clone(), 99).await?;

    println!("Table 42 is active: {}", is_42_active);
    println!("Table 15 is active: {}", is_15_active);
    println!("Table 99 is active: {}", is_99_active);

    // Demonstrate ordering
    place_order(
        store.clone(),
        table_42_id,
        vec![
            Item {
                menu_number: 1,
                description: "Coffee".to_string(),
                price: 4.50,
            },
            Item {
                menu_number: 2,
                description: "Sandwich".to_string(),
                price: 8.00,
            },
        ],
    )
    .await?;

    // Mark items as served
    mark_served(store.clone(), table_42_id, vec![1, 2]).await?;

    // Close the tab
    close_tab(store.clone(), table_42_id, 15.00).await?;

    // Wait for final processing
    sleep(Duration::from_millis(500)).await;

    // Check final state
    let is_42_still_active = is_open(store.clone(), 42).await?;
    println!(
        "Table 42 is still active after closing: {}",
        is_42_still_active
    );

    println!("Cafe example completed successfully!");

    Ok(())
}

async fn on_open(mut store: NatsStore, table_number: u64, waiter: String) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    let tab = Root::<Tab>::new(id);

    let command = TabCommand::Open {
        table_number,
        waiter,
    };
    store.try_write(tab, command).await?;

    Ok(id)
}

async fn place_order(mut store: NatsStore, id: Uuid, items: Vec<Item>) -> anyhow::Result<()> {
    let root = store.read::<Tab>(id).await?;
    let command = TabCommand::PlaceOrder { items };
    store.try_write(root, command).await?;
    Ok(())
}

async fn mark_served(mut store: NatsStore, id: Uuid, menu_numbers: Vec<u64>) -> anyhow::Result<()> {
    let root = store.read::<Tab>(id).await?;
    let command = TabCommand::MarkServed { menu_numbers };
    store.try_write(root, command).await?;
    Ok(())
}

async fn close_tab(mut store: NatsStore, id: Uuid, amount_paid: f64) -> anyhow::Result<()> {
    let root = store.read::<Tab>(id).await?;
    let command = TabCommand::Close { amount_paid };
    store.try_write(root, command).await?;
    Ok(())
}

async fn is_open(store: NatsStore, table_number: u64) -> anyhow::Result<bool> {
    let active_tables = ActiveTables::new();
    store.rebuild(active_tables.clone()).await?;
    Ok(active_tables.is_active(table_number).await)
}
