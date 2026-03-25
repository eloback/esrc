use async_nats::service::ServiceExt;
use futures::StreamExt;
use stream_cancel::Tripwire;
use tracing::instrument;

use crate::aggregate::Aggregate;
use crate::error::{self, Error};
use crate::event::command_service::{CommandError, CommandErrorKind, CommandService};
use crate::event::publish::PublishExt;
use crate::event::replay::ReplayOneExt;
use crate::event::Event;
use crate::version::{DeserializeVersion, SerializeVersion};

use super::NatsStore;

impl CommandService for NatsStore {
    #[instrument(skip_all, level = "debug")]
    async fn serve<A>(&self) -> error::Result<()>
    where
        A: Aggregate,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: serde::de::DeserializeOwned + Send,
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
            .endpoint("command.*")
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        while let Some(request) = endpoint.next().await {
            let subject = request.message.subject.as_str();

            // Extract the UUID from the last token of the subject.
            let id = match subject.rsplit('.').next().and_then(|s| s.parse().ok()) {
                Some(id) => id,
                None => {
                    let err = CommandError::new(
                        CommandErrorKind::InvalidId,
                        format!("could not parse aggregate ID from subject: {subject}"),
                    );
                    reply_error(&request, err).await;
                    continue;
                },
            };

            // Deserialize the command from the request payload.
            let command: A::Command = match serde_json::from_slice(&request.message.payload) {
                Ok(cmd) => cmd,
                Err(e) => {
                    let err = CommandError::new(
                        CommandErrorKind::InvalidPayload,
                        format!("failed to deserialize command: {e}"),
                    );
                    reply_error(&request, err).await;
                    continue;
                },
            };

            // Load the aggregate from sequence 0.
            let root = match self.read::<A>(id).await {
                Ok(root) => root,
                Err(e) => {
                    let err = CommandError::new(
                        CommandErrorKind::Replay,
                        format!("failed to replay aggregate: {e}"),
                    );
                    reply_error(&request, err).await;
                    continue;
                },
            };

            // Process the command and publish the resulting event.
            let mut store = self.clone();
            match store.try_write(root, command, None).await {
                Ok(_) => {
                    if let Err(e) = request.respond(Ok(bytes::Bytes::new())).await {
                        tracing::warn!("failed to send success reply: {e}");
                    }
                },
                Err(Error::Conflict) => {
                    let err = CommandError::new(
                        CommandErrorKind::Conflict,
                        "optimistic concurrency conflict",
                    );
                    reply_error(&request, err).await;
                },
                Err(Error::External(e)) => {
                    let err =
                        CommandError::new(CommandErrorKind::Domain, format!("domain error: {e}"));
                    reply_error(&request, err).await;
                },
                Err(e) => {
                    let err = CommandError::new(
                        CommandErrorKind::Internal,
                        format!("internal error: {e}"),
                    );
                    reply_error(&request, err).await;
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
        A: Aggregate + 'static,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: serde::de::DeserializeOwned + Send,
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
}

async fn reply_error(request: &async_nats::service::Request, error: CommandError) {
    tracing::debug!(
        error_kind = ?error.kind,
        error_message = %error.message,
        "command service replied with error"
    );

    let payload = match serde_json::to_vec(&error) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("failed to serialize CommandError: {e}");
            return;
        },
    };

    if let Err(e) = request.respond(Ok(payload.into())).await {
        tracing::warn!("failed to send error reply: {e}");
    }
}
