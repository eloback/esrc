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
