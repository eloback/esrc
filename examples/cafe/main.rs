#![allow(clippy::unnecessary_literal_unwrap)]
#![allow(unused)]

use std::task;

use async_nats::jetstream;
use esrc::aggregate::Root;
use esrc::event::event_model::{Automation, ViewAutomation};
use esrc::event::{PublishExt, ReplayOne, ReplayOneExt, SubscribeExt};
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

    let mut store = NatsStore::try_new(context, "cafe").await?;
    let task_tracker = tokio_util::task::TaskTracker::new();
    let (exit_tx, mut exit_rx) = tokio::sync::oneshot::channel::<stream_cancel::Trigger>();

    let active_tables = ActiveTables::new();

    let handle = {
        let store = store.clone();
        let task_tracker = task_tracker.clone();
        let active_tables = active_tables.clone();

        task_tracker.clone().spawn(async move {
            store
                .start_view_automation_with_graceful_shutdown(
                    active_tables,
                    "active_tables",
                    task_tracker,
                    exit_tx,
                )
                .await
                .unwrap()
        })
    };

    let id = Uuid::now_v7();
    let tab = Root::<Tab>::new(id);

    let command = TabCommand::Open {
        table_number: 1,
        waiter: "teste".to_string(),
    };
    store.try_write(tab, command).await?;
    let command = TabCommand::Open {
        table_number: 2,
        waiter: "teste".to_string(),
    };
    let root: Root<Tab> = store.read(id).await?;
    let root = store.try_write(root, command).await?;

    let command = TabCommand::Open {
        table_number: 1,
        waiter: "teste".to_string(),
    };

    // Place the `store` and `active_tables` objects inside shared state for
    // your chosen web application / interface framework (such as
    // `axum::extract::State<S>`). Both the NatsStore and any Project impl
    // will be Clone and can be used in this way.

    let table_numbers = active_tables.get_table_numbers().await;
    println!("Active tables: {:#?}", table_numbers);

    let exit = exit_rx.await.expect("Failed to receive exit signal");
    drop(exit);
    task_tracker.close();
    task_tracker.wait().await;

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
