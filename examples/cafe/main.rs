#![allow(clippy::unnecessary_literal_unwrap)]
#![allow(unused)]

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = async_nats::connect("localhost").await?;
    let context = jetstream::new(client);

    let store = NatsStore::try_new(context, "cafe").await?;

    let active_tables = ActiveTables::new();
    store.observe(active_tables).await?;

    // Place the `store` and `active_tables` objects inside shared state for
    // your chosen web application / interface framework (such as
    // `axum::extract::State<S>`). Both the NatsStore and any Project impl
    // will be Clone and can be used in this way.

    Ok(())
}

async fn on_open(table_number: u64, waiter: String) -> anyhow::Result<Uuid> {
    // Replace with actual access to the shared state.
    let shared_store: Result<NatsStore, ()> = Err(());

    let id = Uuid::now_v7();
    let tab = Root::<Tab>::new(id);

    let command = TabCommand::Open {
        table_number,
        waiter,
    };
    shared_store.unwrap().try_write(tab, command).await?;

    Ok(id)
}

async fn is_open(table_number: u64) -> Result<bool, ()> {
    // Replace with actual access to the shared state.
    let shared_project: Result<ActiveTables, ()> = Err(());

    Ok(shared_project.unwrap().is_active(table_number).await)
}
