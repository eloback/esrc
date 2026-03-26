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
