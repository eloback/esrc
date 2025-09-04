use std::collections::HashMap;
use std::sync::Arc;

use esrc::project::{Context, Project};
use esrc::Envelope;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::TabError;
use crate::tab::TabEvent;

#[derive(Clone)]
pub struct ActiveTables {
    table_numbers: Arc<RwLock<HashMap<Uuid, u64>>>,
}

impl ActiveTables {
    pub fn new() -> Self {
        Self {
            table_numbers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn is_active(&self, table_number: u64) -> bool {
        self.table_numbers
            .read()
            .await
            .values()
            .any(|n| *n == table_number)
    }
}

impl Project for ActiveTables {
    type EventGroup = TabEvent;
    type Error = TabError;

    async fn project<'a, E>(
        &mut self,
        context: Context<'a, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: Envelope + Sync,
    {
        let id = Context::id(&context);
        let mut map = self.table_numbers.write().await;

        if let TabEvent::Opened { table_number, .. } = *context {
            map.insert(id, table_number);
        } else if let TabEvent::Closed { .. } = *context {
            map.remove(&id);
        }

        Ok(())
    }
}
