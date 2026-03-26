use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use esrc::envelope::Envelope;
use esrc::project::{Context, Project};
use esrc::query::{Query, QueryHandler};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::OrderEvent;

/// The read model representing a materialized order view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReadModel {
    pub id: Uuid,
    pub item: String,
    pub quantity: u32,
    pub shipped: bool,
}

/// Query enum for the order read model.
#[derive(Debug, Serialize, Deserialize)]
pub enum OrderQuery {
    /// List all orders that have been shipped.
    ListShipped,
    /// List all orders (shipped or not).
    ListAll,
}

impl Query for OrderQuery {
    type ReadModel = OrderReadModel;
    type Response = Vec<OrderReadModel>;
}

/// Shared in-memory storage used by both the projector (write side)
/// and the query handler (read side).
#[derive(Debug, Clone, Default)]
pub struct OrderStore {
    inner: Arc<Mutex<HashMap<Uuid, OrderReadModel>>>,
}

impl OrderStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: &Uuid) -> Option<OrderReadModel> {
        self.inner
            .lock()
            .expect("lock not poisoned")
            .get(id)
            .cloned()
    }

    pub fn upsert(&self, model: OrderReadModel) {
        self.inner
            .lock()
            .expect("lock not poisoned")
            .insert(model.id, model);
    }

    pub fn all(&self) -> Vec<OrderReadModel> {
        self.inner
            .lock()
            .expect("lock not poisoned")
            .values()
            .cloned()
            .collect()
    }
}

/// The projector that writes into the shared OrderStore.
#[derive(Debug, Clone)]
pub struct OrderProjector {
    store: OrderStore,
}

impl OrderProjector {
    pub fn new(store: OrderStore) -> Self {
        Self { store }
    }
}

impl Project for OrderProjector {
    type EventGroup = OrderEvent;
    type Error = std::convert::Infallible;

    async fn project<'de, E: Envelope>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error> {
        let id = Context::id(&context);

        match context.clone() {
            OrderEvent::OrderPlaced { item, quantity } => {
                self.store.upsert(OrderReadModel {
                    id,
                    item: item.clone(),
                    quantity: quantity,
                    shipped: false,
                });
            },
            OrderEvent::OrderShipped => {
                if let Some(mut model) = self.store.get(&id) {
                    model.shipped = true;
                    self.store.upsert(model);
                }
            },
        }

        Ok(())
    }
}

/// The query handler that reads from the shared OrderStore.
#[derive(Debug)]
pub struct OrderQueryHandler {
    store: OrderStore,
}

impl OrderQueryHandler {
    pub fn new(store: OrderStore) -> Self {
        Self { store }
    }
}

impl QueryHandler for OrderQueryHandler {
    type Query = OrderQuery;
    type Id = Uuid;

    async fn get_by_id(&self, id: Self::Id) -> esrc::error::Result<Option<OrderReadModel>> {
        Ok(self.store.get(&id))
    }

    async fn handle(&self, query: Self::Query) -> esrc::error::Result<Vec<OrderReadModel>> {
        match query {
            OrderQuery::ListShipped => {
                Ok(self.store.all().into_iter().filter(|o| o.shipped).collect())
            },
            OrderQuery::ListAll => Ok(self.store.all()),
        }
    }
}
