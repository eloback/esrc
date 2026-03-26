//! Automation slice: reacts to OperationCreated events and sends a notification email command.

use esrc::{
    event::CommandClient,
    event_modeling::{Automation, ComponentName},
    nats::NatsStore,
};
use tracing::instrument;

use crate::domain;

const FEATURE_NAME: &str = "send_notification";

#[derive(Clone)]
pub struct SendNotificationAutomation {
    store: NatsStore,
}

impl esrc::project::Project for SendNotificationAutomation {
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
        SendNotificationAutomation {
            store: store.clone(),
        },
    ));
}
