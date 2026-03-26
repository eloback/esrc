use std::time::Duration;

use esrc::{
    event::CommandClient,
    event_modeling::{Automation, ComponentName},
    nats::NatsStore,
};
use tracing::instrument;

use crate::domain;

#[derive(Clone)]
pub struct SendWelcomeEmailAutomation {
    pub store: NatsStore,
}

impl esrc::project::Project for SendWelcomeEmailAutomation {
    type EventGroup = domain::EmailEvent;
    type Error = esrc::Error;

    #[instrument(skip_all, level = "debug")]
    async fn project<'de, E>(
        &mut self,
        context: esrc::project::Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: esrc::Envelope + Sync,
    {
        if let domain::EmailEvent::WelcomeEmailRequested { email } = &*context {
            tracing::info!(
                aggregate_id = %esrc::project::Context::id(&context),
                email = %email,
                "automation received welcome email request and will mark it sent"
            );

            tokio::time::sleep(Duration::from_millis(150)).await;

            self.store
                .send_command::<domain::EmailAggregate>(
                    esrc::project::Context::id(&context),
                    domain::EmailCommand::MarkWelcomeEmailSent {
                        email: email.clone(),
                    },
                )
                .await?;
        }

        Ok(())
    }
}

pub fn setup(store: NatsStore) {
    store.spawn_automation(
        Automation::new(
            ComponentName::new("examples", "email", "delivery", "send-welcome-email"),
            SendWelcomeEmailAutomation {
                store: store.clone(),
            },
        )
        .max_concurrency(8),
    );
}
