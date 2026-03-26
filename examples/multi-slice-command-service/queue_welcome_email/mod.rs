use esrc::{
    event::CommandClient,
    event_modeling::{Automation, ComponentName},
    nats::NatsStore,
};
use tracing::instrument;

use crate::domain;

#[derive(Clone)]
pub struct QueueWelcomeEmailAutomation {
    store: NatsStore,
}

impl esrc::project::Project for QueueWelcomeEmailAutomation {
    type EventGroup = domain::SignupEvent;
    type Error = esrc::Error;

    #[instrument(skip_all, level = "debug")]
    async fn project<'de, E>(
        &mut self,
        context: esrc::project::Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: esrc::Envelope + Sync,
    {
        if let domain::SignupEvent::SignupRequested { email } = &*context {
            tracing::info!(
                aggregate_id = %esrc::project::Context::id(&context),
                email = %email,
                "automation received signup request and will trigger follow-up commands"
            );

            self.store
                .send_command::<domain::EmailAggregate>(
                    esrc::project::Context::id(&context),
                    domain::EmailCommand::RequestWelcomeEmail {
                        email: email.clone(),
                    },
                )
                .await?;

            self.store
                .send_command::<domain::SignupAggregate>(
                    esrc::project::Context::id(&context),
                    domain::SignupCommand::QueueWelcomeEmail,
                )
                .await?;
        }

        Ok(())
    }
}

pub fn setup(store: NatsStore) {
    store.spawn_automation(Automation::new(
        ComponentName::new("examples", "signup", "onboarding", "queue-welcome-email"),
        QueueWelcomeEmailAutomation {
            store: store.clone(),
        },
    ));
}
