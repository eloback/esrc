use std::marker::PhantomData;

use esrc::aggregate::{Aggregate, Root};
use esrc::error::{self, Error};
use esrc::event::publish::PublishExt;
use esrc::event::replay::ReplayOneExt;
use esrc::nats::NatsStore;
use esrc::version::{DeserializeVersion, SerializeVersion};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::command::CommandHandler;

/// A standard command envelope sent over NATS.
///
/// The command payload wraps the aggregate ID and the serialized command body.
/// Both the ID and the command are encoded as JSON.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandEnvelope<C> {
    /// The ID of the aggregate instance this command targets.
    pub id: Uuid,
    /// The actual command to process.
    pub command: C,
}

/// A standard reply envelope returned after processing a command.
#[derive(Debug, Deserialize, Serialize)]
pub struct CommandReply {
    /// The aggregate ID that was modified.
    pub id: Uuid,
    /// Whether the command succeeded.
    pub success: bool,
    /// An optional message (e.g., error description on failure).
    pub message: Option<String>,
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
            serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;

        let root: Root<A> = store.read(envelope.id).await?;
        let agg_id = envelope.id;
        let root = store.try_write(root, envelope.command, None).await;

        let reply = match root {
            Ok(written) => CommandReply {
                id: Root::id(&written),
                success: true,
                message: None,
            },
            Err(e) => CommandReply {
                id: agg_id,
                success: false,
                message: Some(format!("{e}")),
            },
        };
        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
    }
}
