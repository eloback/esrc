use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Event};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, SerializeVersion, Event)]
pub enum SignupEvent {
    SignupRequested { email: String },
    WelcomeEmailQueued,
}

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, SerializeVersion, Event)]
pub enum EmailEvent {
    WelcomeEmailRequested { email: String },
    WelcomeEmailSent { email: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignupCommand {
    RequestSignup { email: String },
    QueueWelcomeEmail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailCommand {
    RequestWelcomeEmail { email: String },
    MarkWelcomeEmailSent { email: String },
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum SignupError {
    #[error("signup already requested")]
    AlreadyRequested,
    #[error("welcome email already queued")]
    WelcomeAlreadyQueued,
    #[error("welcome email cannot be queued before signup request")]
    MissingSignupRequest,
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum EmailError {
    #[error("welcome email already requested")]
    AlreadyRequested,
    #[error("welcome email already sent")]
    AlreadySent,
    #[error("welcome email cannot be marked sent before request")]
    MissingRequest,
}

#[derive(Debug, Default)]
pub struct SignupAggregate {
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

#[derive(Debug, Default)]
pub struct EmailAggregate {
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
