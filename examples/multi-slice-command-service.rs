use std::time::Duration;

use async_nats::jetstream;
use esrc::aggregate::{Aggregate, Root};
use esrc::event::CommandClient;
use esrc::event_modeling::{Automation, ConsumerName};
use esrc::nats::NatsStore;
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::Event;
use serde::{Deserialize, Serialize};
use tokio::signal;
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, Serialize, SerializeVersion, Event)]
enum SignupEvent {
    SignupRequested { email: String },
    WelcomeEmailQueued,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, Serialize, SerializeVersion, Event)]
enum EmailEvent {
    WelcomeEmailRequested { email: String },
    WelcomeEmailSent { email: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SignupCommand {
    RequestSignup { email: String },
    QueueWelcomeEmail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum EmailCommand {
    RequestWelcomeEmail { email: String },
    MarkWelcomeEmailSent { email: String },
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
enum SignupError {
    #[error("signup already requested")]
    AlreadyRequested,
    #[error("welcome email already queued")]
    WelcomeAlreadyQueued,
    #[error("welcome email cannot be queued before signup request")]
    MissingSignupRequest,
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
enum EmailError {
    #[error("welcome email already requested")]
    AlreadyRequested,
    #[error("welcome email already sent")]
    AlreadySent,
    #[error("welcome email cannot be marked sent before request")]
    MissingRequest,
}

#[derive(Default)]
struct SignupAggregate {
    requested_email: Option<String>,
    welcome_email_queued: bool,
}

impl Aggregate for SignupAggregate {
    type Command = SignupCommand;
    type Event = SignupEvent;
    type Error = SignupError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            SignupCommand::RequestSignup { email } => {
                if self.requested_email.is_some() {
                    Err(SignupError::AlreadyRequested)
                } else {
                    Ok(SignupEvent::SignupRequested { email })
                }
            },
            SignupCommand::QueueWelcomeEmail => {
                if self.requested_email.is_none() {
                    Err(SignupError::MissingSignupRequest)
                } else if self.welcome_email_queued {
                    Err(SignupError::WelcomeAlreadyQueued)
                } else {
                    Ok(SignupEvent::WelcomeEmailQueued)
                }
            },
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            SignupEvent::SignupRequested { email } => {
                self.requested_email = Some(email.clone());
            },
            SignupEvent::WelcomeEmailQueued => {
                self.welcome_email_queued = true;
            },
        }

        self
    }
}

#[derive(Default)]
struct EmailAggregate {
    requested_email: Option<String>,
    sent_email: Option<String>,
}

impl Aggregate for EmailAggregate {
    type Command = EmailCommand;
    type Event = EmailEvent;
    type Error = EmailError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            EmailCommand::RequestWelcomeEmail { email } => {
                if self.requested_email.is_some() {
                    Err(EmailError::AlreadyRequested)
                } else {
                    Ok(EmailEvent::WelcomeEmailRequested { email })
                }
            },
            EmailCommand::MarkWelcomeEmailSent { email } => {
                if self.requested_email.is_none() {
                    Err(EmailError::MissingRequest)
                } else if self.sent_email.is_some() {
                    Err(EmailError::AlreadySent)
                } else {
                    Ok(EmailEvent::WelcomeEmailSent { email })
                }
            },
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            EmailEvent::WelcomeEmailRequested { email } => {
                self.requested_email = Some(email.clone());
            },
            EmailEvent::WelcomeEmailSent { email } => {
                self.sent_email = Some(email.clone());
            },
        }

        self
    }
}

#[derive(Clone)]
struct QueueWelcomeEmailAutomation {
    store: NatsStore,
}

impl esrc::project::Project for QueueWelcomeEmailAutomation {
    type EventGroup = SignupEvent;
    type Error = esrc::Error;

    #[instrument(skip_all, level = "debug")]
    async fn project<'de, E>(
        &mut self,
        context: esrc::project::Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: esrc::Envelope + Sync,
    {
        if let SignupEvent::SignupRequested { email } = &*context {
            tracing::info!(
                aggregate_id = %esrc::project::Context::id(&context),
                email = %email,
                "automation received signup request and will trigger follow-up commands"
            );

            self.store
                .send_command::<EmailAggregate>(
                    esrc::project::Context::id(&context),
                    EmailCommand::RequestWelcomeEmail {
                        email: email.clone(),
                    },
                )
                .await?;

            self.store
                .send_command::<SignupAggregate>(
                    esrc::project::Context::id(&context),
                    SignupCommand::QueueWelcomeEmail,
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(Clone)]
struct SendWelcomeEmailAutomation {
    store: NatsStore,
}

impl esrc::project::Project for SendWelcomeEmailAutomation {
    type EventGroup = EmailEvent;
    type Error = esrc::Error;

    #[instrument(skip_all, level = "debug")]
    async fn project<'de, E>(
        &mut self,
        context: esrc::project::Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: esrc::Envelope + Sync,
    {
        if let EmailEvent::WelcomeEmailRequested { email } = &*context {
            tracing::info!(
                aggregate_id = %esrc::project::Context::id(&context),
                email = %email,
                "automation received welcome email request and will mark it sent"
            );

            tokio::time::sleep(Duration::from_millis(150)).await;

            self.store
                .send_command::<EmailAggregate>(
                    esrc::project::Context::id(&context),
                    EmailCommand::MarkWelcomeEmailSent {
                        email: email.clone(),
                    },
                )
                .await?;
        }

        Ok(())
    }
}

#[instrument(skip_all, level = "info")]
async fn ensure_clean_stream(store: &NatsStore, signup_id: Uuid) -> Result<(), esrc::Error> {
    let _ = store
        .clone()
        .truncate::<SignupEvent>(signup_id, 0_u64.into())
        .await;
    let _ = store
        .clone()
        .truncate::<EmailEvent>(signup_id, 0_u64.into())
        .await;
    Ok(())
}

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
    let store = NatsStore::try_new(jetstream, "esrc-example").await?;

    store.spawn_service::<SignupAggregate>();
    store.spawn_service::<EmailAggregate>();

    store.spawn_automation(Automation::new(
        ConsumerName::new("examples", "signup", "onboarding", "queue-welcome-email"),
        QueueWelcomeEmailAutomation {
            store: store.clone(),
        },
    ));

    store.spawn_automation(
        Automation::new(
            ConsumerName::new("examples", "email", "delivery", "send-welcome-email"),
            SendWelcomeEmailAutomation {
                store: store.clone(),
            },
        )
        .max_concurrency(8),
    );

    let signup_id = Uuid::new_v4();
    ensure_clean_stream(&store, signup_id).await?;

    tracing::info!(aggregate_id = %signup_id, "sending initial signup command");

    store
        .send_command::<SignupAggregate>(
            signup_id,
            SignupCommand::RequestSignup {
                email: "user@example.com".to_owned(),
            },
        )
        .await?;

    let signup_root = store.read::<SignupAggregate>(signup_id).await?;
    let email_root = store.read::<EmailAggregate>(signup_id).await?;

    log_signup_state(&signup_root);
    log_email_state(&email_root);

    tracing::info!("example is running, press ctrl-c to stop");
    signal::ctrl_c().await?;
    store.wait_graceful_shutdown().await;

    Ok(())
}

fn log_signup_state(root: &Root<SignupAggregate>) {
    tracing::info!(
        aggregate_id = %Root::id(root),
        last_sequence = u64::from(Root::last_sequence(root)),
        requested_email = ?root.requested_email,
        welcome_email_queued = root.welcome_email_queued,
        "signup slice state after command handling"
    );
}

fn log_email_state(root: &Root<EmailAggregate>) {
    tracing::info!(
        aggregate_id = %Root::id(root),
        last_sequence = u64::from(Root::last_sequence(root)),
        requested_email = ?root.requested_email,
        sent_email = ?root.sent_email,
        "email slice state after automation chaining"
    );
}
