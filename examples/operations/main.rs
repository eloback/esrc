use std::time::Duration;

use async_nats::jetstream;
use esrc::event::CommandClient;
use esrc::nats::NatsStore;
use esrc::query::QueryClient;
use tokio::signal;
use uuid::Uuid;

use crate::domain::BOUNDED_CONTEXT_NAME;

mod domain;
mod kv_operation_view;
mod memory_operation_list;
mod send_email;
mod send_notification;

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
    let store = NatsStore::try_new(jetstream, BOUNDED_CONTEXT_NAME).await?;

    // -- Command Services --
    store.spawn_service::<domain::operation::OperationAggregate>();
    store.spawn_service::<domain::email::EmailAggregate>();

    // -- Automation: reacts to OperationCreated and sends an email command --
    send_notification::setup(store.clone());

    // -- Read Model (in-memory): materializes a list of operations --
    memory_operation_list::setup(store.clone());

    // -- Read Model (NATS KV): materializes individual operation views --
    kv_operation_view::setup(store.clone()).await?;

    // -- Read Model (in-memory): materializes sent emails --
    send_email::setup(store.clone());

    // Allow consumers and services to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    // =========================================================================
    // Scenario: create two operations, the automation will trigger emails,
    // then we query read models to validate the flow.
    // =========================================================================

    let op_id_1 = Uuid::now_v7();
    let op_id_2 = Uuid::now_v7();

    tracing::info!(op_id = %op_id_1, "creating operation 1");
    store
        .send_command::<domain::operation::OperationAggregate>(
            op_id_1,
            domain::operation::OperationCommand::Create {
                title: "Deploy v2".to_owned(),
                owner: "alice@example.com".to_owned(),
            },
        )
        .await?;

    tracing::info!(op_id = %op_id_2, "creating operation 2");
    store
        .send_command::<domain::operation::OperationAggregate>(
            op_id_2,
            domain::operation::OperationCommand::Create {
                title: "Rollback v1".to_owned(),
                owner: "bob@example.com".to_owned(),
            },
        )
        .await?;

    // Complete operation 1
    tracing::info!(op_id = %op_id_1, "completing operation 1");
    store
        .send_command::<domain::operation::OperationAggregate>(
            op_id_1,
            domain::operation::OperationCommand::Complete,
        )
        .await?;

    // Allow projections and automations to process
    tokio::time::sleep(Duration::from_secs(3)).await;

    // =========================================================================
    // Assertions via QueryClient
    // =========================================================================

    tracing::info!("--- validating in-memory operation list read model ---");

    let all_ops = store
        .query::<memory_operation_list::OperationListQuery>(
            &memory_operation_list::query_name(),
            memory_operation_list::OperationListQuery::ListAll,
        )
        .await?;
    tracing::info!(count = all_ops.len(), "ListAll result");
    assert!(
        all_ops.len() >= 2,
        "expected at least 2 operations, got {}",
        all_ops.len()
    );

    let completed_ops = store
        .query::<memory_operation_list::OperationListQuery>(
            &memory_operation_list::query_name(),
            memory_operation_list::OperationListQuery::ListCompleted,
        )
        .await?;
    tracing::info!(count = completed_ops.len(), "ListCompleted result");
    assert!(
        completed_ops.iter().any(|o| o.id == op_id_1),
        "operation 1 should be in completed list"
    );
    assert!(
        !completed_ops.iter().any(|o| o.id == op_id_2),
        "operation 2 should NOT be in completed list"
    );

    // get_by_id for operation 1
    let op1 = store
        .get_by_id::<memory_operation_list::OperationListQuery, _>(
            &memory_operation_list::query_name(),
            op_id_1,
        )
        .await?;
    assert!(op1.is_some(), "operation 1 should exist in read model");
    let op1 = op1.unwrap();
    tracing::info!(id = %op1.id, title = %op1.title, completed = op1.completed, "operation 1");
    assert!(op1.completed, "operation 1 should be completed");

    // get_by_id for operation 2
    let op2 = store
        .get_by_id::<memory_operation_list::OperationListQuery, _>(
            &memory_operation_list::query_name(),
            op_id_2,
        )
        .await?;
    assert!(op2.is_some(), "operation 2 should exist in read model");
    let op2 = op2.unwrap();
    tracing::info!(id = %op2.id, title = %op2.title, completed = op2.completed, "operation 2");
    assert!(!op2.completed, "operation 2 should NOT be completed");

    tracing::info!("--- validating NATS KV operation view read model ---");

    let kv_op1 = store
        .get_by_id::<kv_operation_view::OperationViewQuery, _>(
            &kv_operation_view::query_name(),
            op_id_1.to_string(),
        )
        .await?;
    assert!(
        kv_op1.is_some(),
        "operation 1 should exist in KV read model"
    );
    let kv_op1 = kv_op1.unwrap();
    tracing::info!(id = %kv_op1.id, title = %kv_op1.title, completed = kv_op1.completed, "KV operation 1");
    assert!(kv_op1.completed, "KV operation 1 should be completed");

    let kv_op2 = store
        .get_by_id::<kv_operation_view::OperationViewQuery, _>(
            &kv_operation_view::query_name(),
            op_id_2.to_string(),
        )
        .await?;
    assert!(
        kv_op2.is_some(),
        "operation 2 should exist in KV read model"
    );
    let kv_op2 = kv_op2.unwrap();
    tracing::info!(id = %kv_op2.id, title = %kv_op2.title, completed = kv_op2.completed, "KV operation 2");
    assert!(!kv_op2.completed, "KV operation 2 should NOT be completed");

    tracing::info!("--- validating send_email read model ---");

    let all_emails = store
        .query::<send_email::SentEmailQuery>(
            &send_email::query_name(),
            send_email::SentEmailQuery::ListAll,
        )
        .await?;
    tracing::info!(count = all_emails.len(), "sent emails");
    assert!(
        all_emails.len() >= 2,
        "expected at least 2 sent emails (one per operation), got {}",
        all_emails.len()
    );
    assert!(
        all_emails
            .iter()
            .any(|e| e.recipient == "alice@example.com"),
        "should have sent email to alice"
    );
    assert!(
        all_emails.iter().any(|e| e.recipient == "bob@example.com"),
        "should have sent email to bob"
    );

    tracing::info!("=== all assertions passed ===");

    tracing::info!("press ctrl-c to stop");
    signal::ctrl_c().await?;
    store.wait_graceful_shutdown().await;

    Ok(())
}
