use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Event};
use serde::{Deserialize, Serialize};

pub const DOMAIN_NAME: &str = "operation";

#[derive(Debug, Clone, Serialize, Deserialize, DeserializeVersion, SerializeVersion, Event)]
pub enum OperationEvent {
    OperationCreated { title: String, owner: String },
    OperationCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationCommand {
    Create { title: String, owner: String },
    Complete,
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum OperationError {
    #[error("operation already exists")]
    AlreadyExists,
    #[error("operation already completed")]
    AlreadyCompleted,
    #[error("operation does not exist")]
    NotCreated,
}

#[derive(Debug, Default)]
pub struct OperationAggregate {
    created: bool,
    completed: bool,
}

impl Aggregate for OperationAggregate {
    type Command = OperationCommand;
    type Event = OperationEvent;
    type Error = OperationError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            OperationCommand::Create { title, owner } => {
                if self.created {
                    Err(OperationError::AlreadyExists)
                } else {
                    Ok(OperationEvent::OperationCreated { title, owner })
                }
            },
            OperationCommand::Complete => {
                if !self.created {
                    Err(OperationError::NotCreated)
                } else if self.completed {
                    Err(OperationError::AlreadyCompleted)
                } else {
                    Ok(OperationEvent::OperationCompleted)
                }
            },
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            OperationEvent::OperationCreated { .. } => {
                self.created = true;
            },
            OperationEvent::OperationCompleted => {
                self.completed = true;
            },
        }
        self
    }
}
