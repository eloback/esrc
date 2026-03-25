use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::error;

impl CommandError {
    /// Map the error kind to a NATS-compatible status code.
    pub fn status_code(&self) -> String {
        match self.kind {
            CommandErrorKind::InvalidId => "400".to_string(),
            CommandErrorKind::InvalidPayload => "400".to_string(),
            CommandErrorKind::Conflict => "409".to_string(),
            CommandErrorKind::Replay => "500".to_string(),
            CommandErrorKind::Domain => "422".to_string(),
            CommandErrorKind::Internal => "500".to_string(),
        }
    }
}

/// A structured error response returned to callers when a command fails.
///
/// This is serialized as JSON in the NATS reply payload alongside
/// a NATS error status header, allowing callers to inspect the failure reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    /// A machine-readable error kind.
    pub kind: CommandErrorKind,
    /// A human-readable description of the error.
    pub message: String,
}

/// The category of a command processing failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandErrorKind {
    /// The aggregate ID in the request subject could not be parsed.
    InvalidId,
    /// The command payload could not be deserialized.
    InvalidPayload,
    /// An optimistic concurrency conflict occurred while publishing the event.
    Conflict,
    /// An error occurred while replaying the aggregate state.
    Replay,
    /// The aggregate's `process` method returned a domain error.
    Domain,
    /// An unexpected internal error.
    Internal,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for CommandError {}

impl CommandError {
    /// Create a new `CommandError` with the given kind and message.
    pub fn new(kind: CommandErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Serve an aggregate as a NATS service endpoint, processing commands.
///
/// Implementations listen for incoming command requests, deserialize the
/// command, load the aggregate via event replay, process the command,
/// publish the resulting event, and reply to the caller.
#[trait_variant::make(Send)]
pub trait CommandService {
    /// Block and serve commands for the given aggregate type.
    ///
    /// The service listens on a subject derived from `A::Event::name()` with
    /// a wildcard for the aggregate UUID (e.g. `<event_name>.*`). On each
    /// request the aggregate is loaded from sequence 0 via `read`, the
    /// command is deserialized and processed via `try_write`, and the caller
    /// receives an empty reply on success or a [`CommandError`] on failure.
    ///
    /// This method runs until the underlying transport is closed or an
    /// unrecoverable error occurs.
    fn serve<A>(&self) -> impl Future<Output = error::Result<()>>
    where
        A: crate::aggregate::Aggregate,
        A::Event: crate::version::SerializeVersion + crate::version::DeserializeVersion,
        A::Command: serde::de::DeserializeOwned + Send;
}
