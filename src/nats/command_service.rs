use async_nats::service::{Request, ServiceExt};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use stream_cancel::Tripwire;
use tracing::instrument;

use crate::aggregate::Aggregate;
use crate::error::{self, Error};
use crate::event::command_service::CommandService;
use crate::event::publish::PublishExt;
use crate::event::replay::ReplayOneExt;
use crate::event::{CommandClient, Event};
use crate::version::{DeserializeVersion, SerializeVersion};

use super::NatsStore;

#[derive(Debug, Serialize, Deserialize)]
pub enum ReplyError<Err> {
    /// An error occurred while sending the reply, e.g. a transport error.
    Internal(String),
    /// An error occurred while serializing the error reply.
    Conflict,
    External(Err),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandReply<E> {
    pub error: Option<ReplyError<E>>,
}

impl CommandService for NatsStore {
    #[instrument(skip_all, level = "debug")]
    async fn serve<A>(&self) -> error::Result<()>
    where
        A: Aggregate + Send + Sync + 'static,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: DeserializeOwned + Send,
        A::Error: std::error::Error + Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let event_name = A::Event::name();

        let service = self
            .client()
            .service_builder()
            .description(format!("Command service for {event_name}"))
            .start(event_name, "0.0.1")
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        let group = service.group(event_name);

        // The endpoint subject uses a wildcard to capture the aggregate UUID
        // from the last token, e.g. `<event_name>.*`.
        let mut endpoint = group
            .endpoint_builder()
            .name("command")
            .add("command.*")
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        while let Some(request) = endpoint.next().await {
            let reply = self.handle_request::<A>(&request).await;
            let bytes = serde_json::to_vec(&reply)
                .map(|bytes| bytes::Bytes::from(bytes))
                .map_err(|e| {
                    Error::Internal(format!("failed to serialize command reply: {e}").into())
                });
            match bytes {
                Ok(bytes) => {
                    request.respond(Ok(bytes)).await.map_err(|e| {
                        Error::Internal(format!("failed to send command reply: {e}").into())
                    })?;
                },
                Err(e) => {
                    tracing::error!("failed to prepare command reply: {e}");
                    // Attempt to send an error reply if serialization failed.
                    let err_reply = CommandReply::<A::Error> {
                        error: Some(ReplyError::Internal(format!(
                            "failed to prepare command reply: {e}"
                        ))),
                    };
                    let err_bytes = serde_json::to_vec(&err_reply)
                        .map(|bytes| bytes::Bytes::from(bytes))
                        .expect("failed to serialize error command reply");
                    request.respond(Ok(err_bytes)).await.unwrap_or_else(|e| {
                        tracing::error!("failed to send error command reply: {e}");
                    });
                },
            }
        }
        Ok(())
    }
}

impl NatsStore {
    /// Spawn `serve` as a background task integrated with graceful shutdown.
    ///
    /// This wraps [`CommandService::serve`] in a tracked, cancellable task
    /// using the `GracefulShutdown` / `TaskTracker` already present on
    /// `NatsStore`. The task will be cancelled when
    /// [`NatsStore::wait_graceful_shutdown`] is called.
    pub fn spawn_service<A>(&self)
    where
        A: Aggregate + Send + Sync + 'static,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: DeserializeOwned + Send,
        A::Error: std::error::Error + Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let store = self.clone();
        let (trigger, tripwire) = Tripwire::new();

        let exit_tx = self.graceful_shutdown.exit_tx.clone();
        let task_tracker = self.graceful_shutdown.task_tracker.clone();

        task_tracker.spawn(async move {
            // Register the trigger so it is cancelled during graceful shutdown.
            if exit_tx.send(trigger).await.is_err() {
                tracing::warn!("failed to register shutdown trigger for command service");
                return;
            }

            tokio::select! {
                result = store.serve::<A>() => {
                    if let Err(e) = result {
                        tracing::error!("command service for '{}' exited with error: {e}", A::Event::name());
                    }
                }
                _ = tripwire => {
                    tracing::info!("command service for '{}' shutting down gracefully", A::Event::name());
                }
            }
        });
    }

    #[instrument(skip_all, level = "debug")]
    pub async fn handle_request<A>(&self, request: &Request) -> CommandReply<A::Error>
    where
        A: Aggregate + Send + Sync + 'static,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: DeserializeOwned + Send,
        A::Error: std::error::Error + Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let subject = request.message.subject.as_str();

        // Extract the UUID from the last token of the subject.
        let id = match subject.rsplit('.').next().and_then(|s| s.parse().ok()) {
            Some(id) => id,
            None => {
                let err = Error::Format(
                    format!(
                        "invalid subject format: expected '<event_name>.<id>', got '{subject}'"
                    )
                    .into(),
                );
                return CommandReply {
                    error: Some(ReplyError::Internal(format!(
                        "invalid subject format: {err}"
                    ))),
                };
            },
        };

        // Deserialize the command from the request payload.
        let command: A::Command = match serde_json::from_slice(&request.message.payload) {
            Ok(cmd) => cmd,
            Err(e) => {
                return CommandReply {
                    error: Some(ReplyError::Internal(format!(
                        "failed to deserialize command: {e}"
                    ))),
                };
            },
        };

        // Load the aggregate from sequence 0.
        let root = match self.read::<A>(id).await {
            Ok(root) => root,
            Err(e) => {
                let err = Error::Internal(format!("failed to load aggregate: {e}").into());
                return CommandReply {
                    error: Some(ReplyError::Internal(format!(
                        "failed to load aggregate: {err}"
                    ))),
                };
            },
        };

        // Process the command and publish the resulting event.
        let mut store = self.clone();
        match store.try_write(root, command, None).await {
            Ok(_) => {
                return CommandReply { error: None };
            },
            Err(Error::Conflict) => {
                return CommandReply {
                    error: Some(ReplyError::Conflict),
                };
            },
            Err(Error::External(e)) => {
                let aggregate_err = e.downcast::<A::Error>();
                return CommandReply {
                    error: match aggregate_err {
                        Ok(agg_err) => Some(ReplyError::External(*agg_err)),
                        Err(e) => Some(ReplyError::Internal(format!("unexpected error type: {e}"))),
                    },
                };
            },
            Err(e) => {
                return CommandReply {
                    error: Some(ReplyError::Internal(format!(
                        "failed to process command: {e}"
                    ))),
                };
            },
        }
    }
}

impl CommandClient for NatsStore {
    async fn send_command<A>(&self, id: uuid::Uuid, command: A::Command) -> error::Result<()>
    where
        A: crate::aggregate::Aggregate + Send + Sync + 'static,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: serde::ser::Serialize + Send,
        A::Error: std::error::Error + Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let subject = format!("{}.{}", A::Event::name(), id);
        let payload = serde_json::to_vec(&command).map_err(|e| {
            Error::Internal(format!("failed to serialize command for sending: {e}").into())
        })?;
        let message = self
            .client()
            .request(subject, payload.into())
            .await
            .map_err(|e| Error::Internal(format!("failed to send command request: {e}").into()))?;
        let reply: CommandReply<A::Error> =
            serde_json::from_slice(&message.payload).map_err(|e| {
                Error::Internal(format!("failed to deserialize command reply: {e}").into())
            })?;
        match reply.error {
            None => Ok(()),
            Some(ReplyError::Internal(e)) => Err(Error::Internal(e.into())),
            Some(ReplyError::Conflict) => Err(Error::Conflict),
            Some(ReplyError::External(e)) => Err(Error::External(Box::new(e))),
        }
    }
}
