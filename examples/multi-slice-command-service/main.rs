use std::time::Duration;

use async_nats::jetstream;
use esrc::aggregate::Root;
use esrc::event::{CommandClient, ReplayOneExt, Truncate};
use esrc::nats::NatsStore;
use tokio::signal;
use tracing::instrument;
use uuid::Uuid;

use crate::domain::{EmailAggregate, EmailEvent, SignupAggregate, SignupEvent};

mod domain;
mod queue_welcome_email;
mod send_welcome_email;

pub const BOUNDED_CONTEXT: &str = "examples";
pub const SIGNUP_DOMAIN: &str = "signup";
pub const EMAIL_DOMAIN: &str = "email";

#[instrument(skip_all, level = "info")]
async fn ensure_clean_stream(store: &NatsStore, signup_id: Uuid) -> Result<(), esrc::Error> {
    let _ = store
        .clone()
        .truncate::<SignupEvent>(signup_id, 0_u64.into())
        .await;
    let _ = store
        .clone()
        .truncate::<EmailEvent>(signup_id, 0_u64.into())
        .await;
    Ok(())
}

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
    let store = NatsStore::try_new(jetstream, "esrc-example").await?;

    // spawn the services before the automations to ensure they are ready to receive commands when
    // the automations start projecting events
    store.spawn_service::<SignupAggregate>();
    store.spawn_service::<EmailAggregate>();

    // setup the slices
    send_welcome_email::setup(store.clone());
    queue_welcome_email::setup(store.clone());

    // give the store some time to spawn the service and automations before sending commands
    tokio::time::sleep(Duration::from_secs(1)).await;

    let signup_id = Uuid::now_v7();
    ensure_clean_stream(&store, signup_id).await?;

    tracing::info!(aggregate_id = %signup_id, "sending initial signup command");

    store
        .send_command::<SignupAggregate>(
            signup_id,
            domain::SignupCommand::RequestSignup {
                email: "user@example.com".to_owned(),
            },
        )
        .await?;

    let signup_root = store.read::<SignupAggregate>(signup_id).await?;
    let email_root = store.read::<EmailAggregate>(signup_id).await?;

    log_aggregate(signup_root);
    log_aggregate(email_root);

    tracing::info!("example is running, press ctrl-c to stop");
    signal::ctrl_c().await?;
    store.wait_graceful_shutdown().await;

    Ok(())
}

/// Helper function to log the state of an aggregate root after command handling.
fn log_aggregate<A: std::fmt::Debug>(root: Root<A>) {
    let id = Root::id(&root);
    let sequence = u64::from(Root::last_sequence(&root));
    let root = Root::into_inner(root);
    tracing::info!(
        aggregate_id = %id,
        last_sequence = %sequence,
        aggregate = ?root,
        "aggregate state after command handling"
    );
}
