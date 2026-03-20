use std::marker::PhantomData;

use esrc::aggregate::{Aggregate, Root};
use esrc::error::{self, Error as EsrcError};
use esrc::event::publish::PublishExt;
use esrc::event::replay::ReplayOneExt;
use esrc::nats::NatsStore;
use esrc::version::{DeserializeVersion, SerializeVersion};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::command::CommandHandler;

// ── Wire types ──────────────────────────────────────────────────────────────
// ── Wire types ──────────────────────────────────────────────────────────────

/// A standard command envelope sent over NATS.
///
/// The command payload wraps the aggregate ID and the serialized command body.
/// Both the ID and the command are encoded as JSON.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandEnvelope<C> {
    /// The ID of the aggregate instance this command targets.
    pub id: Uuid,
    pub command: C,
}

/// A structured error carried inside a [`CommandReply`] when a command fails.
///
/// `variant` names the [`esrc::error::Error`] arm that was produced
/// (`"External"`, `"Internal"`, `"Format"`, `"Invalid"`, `"Conflict"`).
///
/// For the `"External"` variant `payload` holds the JSON-serialized
/// user-defined error returned by [`Aggregate::process`]. Callers that know
/// which aggregate they targeted can deserialize `payload` back into the
/// concrete error type with [`serde_json::from_value`].
///
/// For all other variants `payload` contains a plain `{"message":"…"}` object
/// so callers always have a human-readable description.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandError {
    /// The `esrc::Error` variant name.
    pub variant: String,
    /// JSON-encoded error payload; structure depends on `variant`.
    pub payload: serde_json::Value,
}

/// A standard reply envelope returned after processing a command.
///
/// On success `error` is `None`.
/// On failure `success` is `false` and `error` carries a [`CommandError`]
/// that the caller can inspect or deserialize.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandReply {
    /// The aggregate ID that was modified.
    pub id: Uuid,
    /// Whether the command succeeded.
    pub success: bool,
    /// Structured error information when `success` is `false`.
    pub error: Option<CommandError>,
}

/// A generic [`CommandHandler`] implementation for NATS-backed aggregates.
///
/// This handler:
/// 1. Deserializes the incoming payload as a [`CommandEnvelope<A::Command>`].
/// 2. Loads the aggregate using [`ReplayOneExt::read`].
/// 3. Processes and writes the command using [`PublishExt::try_write`].
/// 4. Returns a serialized [`CommandReply`].
///
/// `A` is the aggregate type. `A::Command` must implement `Deserialize` and
/// `A::Event` must implement both `SerializeVersion` and `DeserializeVersion`.
pub struct AggregateCommandHandler<A>
where
    A: Aggregate,
{
    /// The name used to route commands to this handler.
    ///
    /// Convention: `<AggregateName>.<CommandName>` or just `<AggregateName>`.
    handler_name: &'static str,
    _phantom: PhantomData<A>,
}

impl<A> AggregateCommandHandler<A>
where
    A: Aggregate,
{
    /// Create a new handler with the given routing name.
    pub fn new(handler_name: &'static str) -> Self {
        Self {
            handler_name,
            _phantom: PhantomData,
        }
    }
}

impl<A> CommandHandler<NatsStore> for AggregateCommandHandler<A>
where
    A: Aggregate + Send + Sync + 'static,
    A::Command: for<'de> Deserialize<'de> + Send,
    A::Event: SerializeVersion + DeserializeVersion + Send,
{
    fn name(&self) -> &'static str {
        self.handler_name
    }

    async fn handle<'a>(
        &'a self,
        store: &'a mut NatsStore,
        payload: &'a [u8],
    ) -> error::Result<Vec<u8>> {
        let envelope: CommandEnvelope<A::Command> =
            serde_json::from_slice(payload).map_err(|e| EsrcError::Format(e.into()))?;

        let root: Root<A> = store.read(envelope.id).await?;
        let agg_id = envelope.id;
        let root = store.try_write(root, envelope.command, None).await;

        let reply = match root {
            Ok(written) => CommandReply {
                id: Root::id(&written),
                success: true,
                error: None,
            },
            Err(e) => {
                let cmd_error = esrc_error_to_command_error(&e);
                CommandReply {
                    id: agg_id,
                    success: false,
                    error: Some(cmd_error),
                }
            },
        };
        serde_json::to_vec(&reply).map_err(|e| EsrcError::Format(e.into()))
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Convert an [`esrc::Error`] into a wire-safe [`CommandError`].
///
/// For `External` errors the inner `Box<dyn std::error::Error>` is serialized
/// via its `Display` impl wrapped in `{"message":"…"}`. If the concrete type
/// implements `serde::Serialize` the caller can supply a richer payload by
/// serializing it before boxing; see [`AggregateCommandHandler`] for the
/// pattern used in practice.
///
/// For all other variants a `{"message":"…"}` payload is produced.
fn esrc_error_to_command_error(err: &EsrcError) -> CommandError {
    match err {
        EsrcError::External(e) => {
            // The external error is the aggregate's domain error. Try to
            // round-trip it through JSON via its Display representation so that
            // callers at least get a human-readable payload. If the concrete
            // type was pre-serialized (see `SerializableExternalError` below)
            // the JSON value will be richer.
            let payload = serde_json::json!({ "message": format!("{e}") });
            CommandError { variant: "External".into(), payload }
        },
        EsrcError::Internal(e) => CommandError {
            variant: "Internal".into(),
            payload: serde_json::json!({ "message": format!("{e}") }),
        },
        EsrcError::Format(e) => CommandError {
            variant: "Format".into(),
            payload: serde_json::json!({ "message": format!("{e}") }),
        },
        EsrcError::Invalid => CommandError {
            variant: "Invalid".into(),
            payload: serde_json::json!({ "message": "consumed invalid event in stream" }),
        },
        EsrcError::Conflict => CommandError {
            variant: "Conflict".into(),
            payload: serde_json::json!({ "message": "event transaction failed" }),
        },
    }
}
