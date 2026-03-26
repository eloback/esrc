use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Event};
use serde::{Deserialize, Serialize};

pub const DOMAIN_NAME: &str = "email";

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, SerializeVersion, Event)]
pub enum EmailEvent {
    NotificationEmailSent { recipient: String, subject: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailCommand {
    SendNotification { recipient: String, subject: String },
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum EmailError {
    #[error("email already sent for this aggregate")]
    AlreadySent,
}

#[derive(Debug, Default)]
pub struct EmailAggregate {
    sent: bool,
}

impl Aggregate for EmailAggregate {
    type Command = EmailCommand;
    type Event = EmailEvent;
    type Error = EmailError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            EmailCommand::SendNotification { recipient, subject } => {
                if self.sent {
                    Err(EmailError::AlreadySent)
                } else {
                    Ok(EmailEvent::NotificationEmailSent { recipient, subject })
                }
            },
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            EmailEvent::NotificationEmailSent { .. } => {
                self.sent = true;
            },
        }
        self
    }
}
