====
>   Info: Duration: 1m 22s | ~$0.3535 | Prompt Tokens: 34,658 (cached: 0, cache_creation: 0) | Completion Tokens: 7,207 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 15 | Working Files: 0 | Knowledge Files: 4
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.



Looking at the existing examples and the codebase, I'll create a comprehensive modular example that tests all the required components.

<FILE_CHANGES>

<FILE_NEW file_path="examples/full-slice-example/main.rs">
use std::time::Duration;

use async_nats::jetstream;
use esrc::event::CommandClient;
use esrc::nats::NatsStore;
use esrc::query::QueryClient;
use tokio::signal;
use uuid::Uuid;

mod domain;
mod create_operation;
mod send_email;
mod memory_operation_list;
mod kv_operation_view;

pub const BOUNDED_CONTEXT: &str = "fullexample";

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
    let store = NatsStore::try_new(jetstream, "esrc-full-example").await?;

    // -- Command Services --
    store.spawn_service::<domain::operation::OperationAggregate>();
    store.spawn_service::<domain::email::EmailAggregate>();

    // -- Automation: reacts to OperationCreated and sends an email command --
    create_operation::setup(store.clone());

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
    assert!(kv_op1.is_some(), "operation 1 should exist in KV read model");
    let kv_op1 = kv_op1.unwrap();
    tracing::info!(id = %kv_op1.id, title = %kv_op1.title, completed = kv_op1.completed, "KV operation 1");
    assert!(kv_op1.completed, "KV operation 1 should be completed");

    let kv_op2 = store
        .get_by_id::<kv_operation_view::OperationViewQuery, _>(
            &kv_operation_view::query_name(),
            op_id_2.to_string(),
        )
        .await?;
    assert!(kv_op2.is_some(), "operation 2 should exist in KV read model");
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
        all_emails.iter().any(|e| e.recipient == "alice@example.com"),
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
</FILE_NEW>

<FILE_NEW file_path="examples/full-slice-example/domain/mod.rs">
pub mod operation;
pub mod email;

pub const BOUNDED_CONTEXT_NAME: &str = "fullexample";
</FILE_NEW>

<FILE_NEW file_path="examples/full-slice-example/domain/operation.rs">
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Event};
use serde::{Deserialize, Serialize};

pub const DOMAIN_NAME: &str = "operation";

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, SerializeVersion, Event)]
pub enum OperationEvent {
    OperationCreated { title: String, owner: String },
    OperationCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationCommand {
    Create { title: String, owner: String },
    Complete,
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum OperationError {
    #[error("operation already exists")]
    AlreadyExists,
    #[error("operation already completed")]
    AlreadyCompleted,
    #[error("operation does not exist")]
    NotCreated,
}

#[derive(Debug, Default)]
pub struct OperationAggregate {
    created: bool,
    completed: bool,
}

impl Aggregate for OperationAggregate {
    type Command = OperationCommand;
    type Event = OperationEvent;
    type Error = OperationError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            OperationCommand::Create { title, owner } => {
                if self.created {
                    Err(OperationError::AlreadyExists)
                } else {
                    Ok(OperationEvent::OperationCreated { title, owner })
                }
            },
            OperationCommand::Complete => {
                if !self.created {
                    Err(OperationError::NotCreated)
                } else if self.completed {
                    Err(OperationError::AlreadyCompleted)
                } else {
                    Ok(OperationEvent::OperationCompleted)
                }
            },
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            OperationEvent::OperationCreated { .. } => {
                self.created = true;
            },
            OperationEvent::OperationCompleted => {
                self.completed = true;
            },
        }
        self
    }
}
</FILE_NEW>

<FILE_NEW file_path="examples/full-slice-example/domain/email.rs">
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Event};
use serde::{Deserialize, Serialize};

pub const DOMAIN_NAME: &str = "email";

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, SerializeVersion, Event)]
pub enum EmailEvent {
    NotificationEmailSent { recipient: String, subject: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailCommand {
    SendNotification { recipient: String, subject: String },
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum EmailError {
    #[error("email already sent for this aggregate")]
    AlreadySent,
}

#[derive(Debug, Default)]
pub struct EmailAggregate {
    sent: bool,
}

impl Aggregate for EmailAggregate {
    type Command = EmailCommand;
    type Event = EmailEvent;
    type Error = EmailError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            EmailCommand::SendNotification { recipient, subject } => {
                if self.sent {
                    Err(EmailError::AlreadySent)
                } else {
                    Ok(EmailEvent::NotificationEmailSent { recipient, subject })
                }
            },
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            EmailEvent::NotificationEmailSent { .. } => {
                self.sent = true;
            },
        }
        self
    }
}
</FILE_NEW>

<FILE_NEW file_path="examples/full-slice-example/create_operation/mod.rs">
//! Automation slice: reacts to OperationCreated events and sends a notification email command.

use esrc::{
    event::CommandClient,
    event_modeling::{Automation, ComponentName},
    nats::NatsStore,
};
use tracing::instrument;

use crate::domain;

const FEATURE_NAME: &str = "create_operation";

#[derive(Clone)]
pub struct CreateOperationAutomation {
    store: NatsStore,
}

impl esrc::project::Project for CreateOperationAutomation {
    type EventGroup = domain::operation::OperationEvent;
    type Error = esrc::Error;

    #[instrument(skip_all, level = "debug")]
    async fn project<'de, E>(
        &mut self,
        context: esrc::project::Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: esrc::Envelope + Sync,
    {
        if let domain::operation::OperationEvent::OperationCreated { owner, title } = &*context {
            let aggregate_id = esrc::project::Context::id(&context);
            tracing::info!(
                aggregate_id = %aggregate_id,
                owner = %owner,
                title = %title,
                "automation: operation created, sending notification email"
            );

            self.store
                .send_command::<domain::email::EmailAggregate>(
                    aggregate_id,
                    domain::email::EmailCommand::SendNotification {
                        recipient: owner.clone(),
                        subject: format!("Operation '{}' created", title),
                    },
                )
                .await?;
        }

        Ok(())
    }
}

pub fn setup(store: NatsStore) {
    store.spawn_automation(Automation::new(
        ComponentName::new(
            domain::BOUNDED_CONTEXT_NAME,
            domain::operation::DOMAIN_NAME,
            FEATURE_NAME,
            "send-notification",
        ),
        CreateOperationAutomation {
            store: store.clone(),
        },
    ));
}
</FILE_NEW>

<FILE_NEW file_path="examples/full-slice-example/memory_operation_list/mod.rs">
//! In-memory read model slice: materializes a queryable list of operations.

use esrc::envelope::Envelope;
use esrc::event_modeling::{ComponentName, ReadModelSlice};
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use esrc::query::in_memory::InMemoryViewStore;
use esrc::query::{Query, QueryTransport};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain;

const FEATURE_NAME: &str = "memory_operation_list";

/// The read model representing a materialized operation list entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationListReadModel {
    pub id: Uuid,
    pub title: String,
    pub owner: String,
    pub completed: bool,
}

/// Query enum for the in-memory operation list.
#[derive(Debug, Serialize, Deserialize)]
pub enum OperationListQuery {
    ListAll,
    ListCompleted,
}

impl Query for OperationListQuery {
    type ReadModel = OperationListReadModel;
    type Response = Vec<OperationListReadModel>;
}

/// Projector that writes into the shared InMemoryViewStore.
#[derive(Clone)]
pub struct OperationListProjector {
    view_store: InMemoryViewStore<OperationListReadModel, OperationListQuery>,
}

impl Project for OperationListProjector {
    type EventGroup = domain::operation::OperationEvent;
    type Error = std::convert::Infallible;

    async fn project<'de, E: Envelope>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error> {
        let id = Context::id(&context);

        match context.clone() {
            domain::operation::OperationEvent::OperationCreated { title, owner } => {
                self.view_store.upsert(
                    id,
                    OperationListReadModel {
                        id,
                        title,
                        owner,
                        completed: false,
                    },
                );
            },
            domain::operation::OperationEvent::OperationCompleted => {
                if let Some(mut model) = self.view_store.get(&id) {
                    model.completed = true;
                    self.view_store.upsert(id, model);
                }
            },
        }

        Ok(())
    }
}

/// Returns the ComponentName used for this slice query service.
pub fn query_name() -> ComponentName {
    ComponentName::new(
        domain::BOUNDED_CONTEXT_NAME,
        domain::operation::DOMAIN_NAME,
        FEATURE_NAME,
        "query",
    )
}

pub fn setup(store: NatsStore) {
    let view_store: InMemoryViewStore<OperationListReadModel, OperationListQuery> =
        InMemoryViewStore::new(|query, snapshot| match query {
            OperationListQuery::ListAll => Ok(snapshot.values().cloned().collect()),
            OperationListQuery::ListCompleted => Ok(snapshot
                .values()
                .filter(|o| o.completed)
                .cloned()
                .collect()),
        });

    let projector = OperationListProjector {
        view_store: view_store.clone(),
    };

    let name = ComponentName::new(
        domain::BOUNDED_CONTEXT_NAME,
        domain::operation::DOMAIN_NAME,
        FEATURE_NAME,
        "query",
    );

    let slice = ReadModelSlice::new(name, projector, view_store)
        .with_query_transport(QueryTransport::NatsRequestReply);

    store.spawn_read_model_slice(slice);
}
</FILE_NEW>

<FILE_NEW file_path="examples/full-slice-example/kv_operation_view/mod.rs">
//! NATS KV read model slice: materializes individual operation views in a KV bucket.

use esrc::envelope::Envelope;
use esrc::event_modeling::{ComponentName, ReadModelSlice};
use esrc::nats::query_kv::NatsKvStore;
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use esrc::query::{Query, QueryTransport};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain;

const FEATURE_NAME: &str = "kv_operation_view";

/// The read model representing a single operation stored in NATS KV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationViewReadModel {
    pub id: Uuid,
    pub title: String,
    pub owner: String,
    pub completed: bool,
}

/// Query enum for the KV operation view.
#[derive(Debug, Serialize, Deserialize)]
pub enum OperationViewQuery {
    /// Placeholder; individual lookups use get_by_id with a String key.
    Noop,
}

impl Query for OperationViewQuery {
    type ReadModel = OperationViewReadModel;
    type Response = Vec<OperationViewReadModel>;
}

/// Projector that writes into the NatsKvStore.
#[derive(Clone)]
pub struct OperationViewProjector {
    kv_store: NatsKvStore<OperationViewReadModel, OperationViewQuery>,
}

impl Project for OperationViewProjector {
    type EventGroup = domain::operation::OperationEvent;
    type Error = esrc::Error;

    async fn project<'de, E: Envelope>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error> {
        let id = Context::id(&context);
        let key = id.to_string();

        match context.clone() {
            domain::operation::OperationEvent::OperationCreated { title, owner } => {
                self.kv_store
                    .put(
                        &key,
                        &OperationViewReadModel {
                            id,
                            title,
                            owner,
                            completed: false,
                        },
                    )
                    .await?;
            },
            domain::operation::OperationEvent::OperationCompleted => {
                if let Some(mut model) = self.kv_store.get(&key).await? {
                    model.completed = true;
                    self.kv_store.put(&key, &model).await?;
                }
            },
        }

        Ok(())
    }
}

/// Returns the ComponentName used for this slice query service.
pub fn query_name() -> ComponentName {
    ComponentName::new(
        domain::BOUNDED_CONTEXT_NAME,
        domain::operation::DOMAIN_NAME,
        FEATURE_NAME,
        "query",
    )
}

pub async fn setup(store: NatsStore) -> Result<(), esrc::Error> {
    let name = ComponentName::new(
        domain::BOUNDED_CONTEXT_NAME,
        domain::operation::DOMAIN_NAME,
        FEATURE_NAME,
        "query",
    );

    let kv_store: NatsKvStore<OperationViewReadModel, OperationViewQuery> =
        NatsKvStore::new(&store, &name, |_query, _kv| {
            Box::pin(async move {
                // get_by_id handles individual lookups; Noop returns empty.
                Ok(vec![])
            })
        })
        .await?;

    let projector = OperationViewProjector {
        kv_store: kv_store.clone(),
    };

    let slice = ReadModelSlice::new(name, projector, kv_store)
        .with_query_transport(QueryTransport::NatsRequestReply);

    store.spawn_read_model_slice(slice);

    Ok(())
}
</FILE_NEW>

<FILE_NEW file_path="examples/full-slice-example/send_email/mod.rs">
//! In-memory read model slice: materializes sent email records from EmailEvent.

use esrc::envelope::Envelope;
use esrc::event_modeling::{ComponentName, ReadModelSlice};
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use esrc::query::in_memory::InMemoryViewStore;
use esrc::query::{Query, QueryTransport};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain;

const FEATURE_NAME: &str = "send_email";

/// The read model representing a sent email record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentEmailReadModel {
    pub id: Uuid,
    pub recipient: String,
    pub subject: String,
}

/// Query enum for the sent email read model.
#[derive(Debug, Serialize, Deserialize)]
pub enum SentEmailQuery {
    ListAll,
}

impl Query for SentEmailQuery {
    type ReadModel = SentEmailReadModel;
    type Response = Vec<SentEmailReadModel>;
}

/// Projector that writes into the shared InMemoryViewStore.
#[derive(Clone)]
pub struct SentEmailProjector {
    view_store: InMemoryViewStore<SentEmailReadModel, SentEmailQuery>,
}

impl Project for SentEmailProjector {
    type EventGroup = domain::email::EmailEvent;
    type Error = std::convert::Infallible;

    async fn project<'de, E: Envelope>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error> {
        let id = Context::id(&context);

        match context.clone() {
            domain::email::EmailEvent::NotificationEmailSent { recipient, subject } => {
                self.view_store.upsert(
                    id,
                    SentEmailReadModel {
                        id,
                        recipient,
                        subject,
                    },
                );
            },
        }

        Ok(())
    }
}

/// Returns the ComponentName used for this slice query service.
pub fn query_name() -> ComponentName {
    ComponentName::new(
        domain::BOUNDED_CONTEXT_NAME,
        domain::email::DOMAIN_NAME,
        FEATURE_NAME,
        "query",
    )
}

pub fn setup(store: NatsStore) {
    let view_store: InMemoryViewStore<SentEmailReadModel, SentEmailQuery> =
        InMemoryViewStore::new(|query, snapshot| match query {
            SentEmailQuery::ListAll => Ok(snapshot.values().cloned().collect()),
        });

    let projector = SentEmailProjector {
        view_store: view_store.clone(),
    };

    let name = ComponentName::new(
        domain::BOUNDED_CONTEXT_NAME,
        domain::email::DOMAIN_NAME,
        FEATURE_NAME,
        "query",
    );

    let slice = ReadModelSlice::new(name, projector, view_store)
        .with_query_transport(QueryTransport::NatsRequestReply);

    store.spawn_read_model_slice(slice);
}
</FILE_NEW>

</FILE_CHANGES>

Here is a summary of the structure:

- **`domain/`** - Contains aggregates, events, and commands split into `operation.rs` and `email.rs`, with `mod.rs` exposing `BOUNDED_CONTEXT_NAME`.
- **`create_operation/`** - Automation slice that reacts to `OperationCreated` events and sends a `SendNotification` command to the `EmailAggregate`.
- **`memory_operation_list/`** - In-memory read model slice using `InMemoryViewStore` + `ReadModelSlice` to materialize and query operations.
- **`kv_operation_view/`** - NATS KV read model slice using `NatsKvStore` + `ReadModelSlice` to persist operation views in a JetStream KV bucket.
- **`send_email/`** - In-memory read model slice that materializes `EmailEvent` records, proving the automation triggered the email command successfully.
- **`main.rs`** - Orchestrates everything: spawns command services, automation, both read model types, sends commands via `CommandClient`, then queries via `QueryClient` with assertions to validate the entire flow.

<suggested_git_command>
git add -A . && git commit -m "+ full-slice-example - Add comprehensive example testing command service, automation, in-memory and KV read models, command client, and query client"
</suggested_git_command>

