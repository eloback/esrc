use crate::aggregate::Aggregate;
use crate::error;

/// A structured error response returned to the caller when a command fails.
///
/// This type is serialized as JSON and sent as the reply payload when the
/// service encounters an error processing a command. The caller can
/// deserialize this to inspect the failure reason.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CommandError {
    /// A short machine-readable error kind.
    pub kind: CommandErrorKind,
    /// A human-readable description of the error.
    pub message: String,
}

/// The category of error that caused a command to fail.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandErrorKind {
    /// The aggregate ID embedded in the request subject was missing or invalid.
    InvalidSubject,
    /// The request payload could not be deserialized into the command type.
    InvalidPayload,
    /// The aggregate could not be loaded from the event store.
    LoadFailed,
    /// The command was rejected by the aggregate's own validation logic.
    CommandFailed,
    /// An optimistic concurrency conflict occurred while publishing the event.
    Conflict,
    /// An unexpected internal error occurred.
    Internal,
}

impl CommandError {
    /// Construct a `CommandError` with the given kind and message.
    pub fn new(kind: CommandErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for CommandError {}

/// Serve incoming commands for an aggregate over a transport endpoint.
///
/// Implementors listen for serialized commands on a subject derived from the
/// aggregate's event name, reconstruct the aggregate state via replay, process
/// the command, publish the resulting event, and reply to the caller.
///
/// The method blocks until the service is shut down or an unrecoverable error
/// occurs.
#[trait_variant::make(Send)]
pub trait CommandService {
    /// Run the command service loop for the given aggregate type.
    ///
    /// The service listens for commands targeting aggregates of type `A`. The
    /// aggregate ID is extracted from the request subject. On each request the
    /// aggregate is loaded from sequence 0 via replay, the command is
    /// deserialized from the payload, processed via `try_write`, and an
    /// empty ack or a structured [`CommandError`] is sent back as the reply.
    ///
    /// `Aggregate::Command` must implement `serde::de::DeserializeOwned`.
    async fn serve<A>(&mut self) -> error::Result<()>
    where
        A: Aggregate,
        A::Command: serde::de::DeserializeOwned;
}
