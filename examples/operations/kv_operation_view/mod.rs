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
