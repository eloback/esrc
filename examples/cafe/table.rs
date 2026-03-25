use std::collections::HashMap;
use std::sync::Arc;

use esrc::envelope::TryFromEnvelope;
use esrc::project::{Context, Project};
use esrc::{Envelope, EventGroup};
use esrc_derive::{DeserializeVersion, SerializeVersion};
use serde::{Deserialize, Serialize};
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

#[derive(
    Deserialize,
    DeserializeVersion,
    Serialize,
    SerializeVersion,
    PartialEq,
    Debug,
    Clone,
    TryFromEnvelope,
)]
pub enum TabProjectionEvents {
    TabEvent(TabEvent),
}
impl EventGroup for TabProjectionEvents {
    fn names() -> impl Iterator<Item = &'static str> {
        ["TabProjection"].into_iter()
    }
}

impl<'a> Project<'a> for ActiveTables {
    type EventGroup = TabProjectionEvents;
    type Error = TabError;

    async fn project<E>(
        &mut self,
        context: Context<'a, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: Envelope + Sync,
    {
        let id = Context::id(&context);
        let mut map = self.table_numbers.write().await;

        match (*context).clone() {
            TabProjectionEvents::TabEvent(event) => {
                if let TabEvent::Opened { table_number, .. } = event {
                    map.insert(id, table_number);
                } else if let TabEvent::Closed { .. } = event {
                    map.remove(&id);
                }
            },
            _ => (),
        }

        Ok(())
    }
}
