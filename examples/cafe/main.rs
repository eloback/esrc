#![allow(clippy::unnecessary_literal_unwrap)]
#![allow(unused)]

use std::sync::Arc;

use async_nats::jetstream;
use esrc::aggregate::Root;
use esrc::event::{PublishExt, ReplayOneExt, SubscribeExt};
use esrc::nats::NatsStore;
use tab::{Tab, TabCommand};
use table::ActiveTables;
use uuid::Uuid;

mod error;
mod tab;
mod table;

#[derive(Clone)]
struct SharedState {
    store: NatsStore,
    active_tables: ActiveTables,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = async_nats::connect("localhost").await?;
    let context = jetstream::new(client);

    let store = NatsStore::try_new(context, "cafe").await?;

    let active_tables = ActiveTables::new();

    start_observer(store.clone(), active_tables.clone());

    let shared_state = SharedState {
        store: store.clone(),
        active_tables: active_tables.clone(),
    };

    println!(
        "Is table 1 open? {}",
        is_open(shared_state.clone(), 1).await?
    );

    on_open(shared_state.clone(), 1, "Alice".to_string()).await?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    println!(
        "Is table 1 open? {}",
        is_open(shared_state.clone(), 1).await?
    );

    println!(
        "Is table 2 open? {}",
        is_open(shared_state.clone(), 2).await?
    );

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");

    Ok(())
}

fn start_observer<S, P>(store: S, active_tables: P)
where
    S: SubscribeExt + Send + Sync + 'static,
    P: for<'de> esrc::project::Project<'de> + Send + 'static,
{
    tokio::spawn(async move {
        if let Err(e) = store.observe(active_tables).await {
            eprintln!("Error observing events: {e}");
        }
    });
}

async fn on_open(
    shared_state: SharedState,
    table_number: u64,
    waiter: String,
) -> anyhow::Result<Uuid> {
    // Replace with actual access to the shared state.
    let mut shared_store = shared_state.store;

    let id = Uuid::now_v7();
    let tab = Root::<Tab>::new(id);

    let command = TabCommand::Open {
        table_number,
        waiter,
    };
    shared_store.try_write(tab, command).await?;

    Ok(id)
}

async fn is_open(shared_state: SharedState, table_number: u64) -> anyhow::Result<bool> {
    // Replace with actual access to the shared state.
    let shared_project = shared_state.active_tables;

    Ok(shared_project.is_active(table_number).await)
}
