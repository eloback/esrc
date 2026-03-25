use async_nats::service::ServiceExt;
use tracing::instrument;
use uuid::Uuid;

use super::subject::NatsSubject;
use super::NatsStore;
use crate::aggregate::Aggregate;
use crate::error::{self, Error};
use crate::event::command_service::{CommandError, CommandErrorKind};
use crate::event::{CommandService, CommandServiceExt};
use crate::event::{PublishExt, ReplayOneExt};

impl CommandService for NatsStore {
    #[instrument(skip_all, level = "debug")]
    async fn serve<A>(&mut self) -> error::Result<()>
    where
        A: Aggregate,
        A::Command: serde::de::DeserializeOwned,
    {
        let service_name = A::Event::name().to_owned();
        let endpoint_subject = NatsSubject::Event(A::Event::name().into()).into_string("");
        // Strip leading dot that into_string adds when prefix is empty.
        let endpoint_subject = endpoint_subject.trim_start_matches('.').to_owned();

        let service = self
            .client()
            .service_builder()
            .start(&service_name, "0.1.0")
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        let mut endpoint = service
            .endpoint(&endpoint_subject)
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        while let Some(request) = endpoint.next().await {
            // Extract UUID from the last subject token, e.g. "<event_name>.<uuid>".
            let id: Uuid = match request.subject.split('.').last() {
                Some(token) => match Uuid::try_parse(token) {
                    Ok(id) => id,
                    Err(_) => {
                        let err = CommandError::new(
                            CommandErrorKind::InvalidSubject,
                            format!("subject token is not a valid UUID: {token}"),
                        );
                        let _ = request
                            .error(Some("invalid_subject"), &serialize_error(&err))
                            .await;
                        continue;
                    },
                },
                None => {
                    let err = CommandError::new(
                        CommandErrorKind::InvalidSubject,
                        "request subject is missing aggregate ID token",
                    );
                    let _ = request
                        .error(Some("invalid_subject"), &serialize_error(&err))
                        .await;
                    continue;
                },
            };

            let command: A::Command = match serde_json::from_slice(&request.message.payload) {
                Ok(cmd) => cmd,
                Err(e) => {
                    let err = CommandError::new(
                        CommandErrorKind::InvalidPayload,
                        format!("failed to deserialize command: {e}"),
                    );
                    let _ = request
                        .error(Some("invalid_payload"), &serialize_error(&err))
                        .await;
                    continue;
                },
            };

            let root = match self.read::<A>(id).await {
                Ok(r) => r,
                Err(e) => {
                    let err = CommandError::new(
                        CommandErrorKind::LoadFailed,
                        format!("failed to load aggregate: {e}"),
                    );
                    let _ = request
                        .error(Some("load_failed"), &serialize_error(&err))
                        .await;
                    continue;
                },
            };

            match self.try_write::<A>(root, command, None).await {
                Ok(_) => {
                    let _ = request.reply(bytes::Bytes::new()).await;
                },
                Err(Error::Conflict) => {
                    let err = CommandError::new(
                        CommandErrorKind::Conflict,
                        "optimistic concurrency conflict; please retry",
                    );
                    let _ = request
                        .error(Some("conflict"), &serialize_error(&err))
                        .await;
                },
                Err(Error::External(e)) => {
                    let err = CommandError::new(
                        CommandErrorKind::CommandFailed,
                        format!("command rejected: {e}"),
                    );
                    let _ = request
                        .error(Some("command_failed"), &serialize_error(&err))
                        .await;
                },
                Err(e) => {
                    let err = CommandError::new(
                        CommandErrorKind::Internal,
                        format!("internal error: {e}"),
                    );
                    let _ = request
                        .error(Some("internal"), &serialize_error(&err))
                        .await;
                },
            }
        }

        Ok(())
    }
}

fn serialize_error(err: &CommandError) -> bytes::Bytes {
    serde_json::to_vec(err)
        .map(bytes::Bytes::from)
        .unwrap_or_else(|_| {
            bytes::Bytes::from_static(
                b"{\"kind\":\"internal\",\"message\":\"serialization error\"}",
            )
        })
}

impl CommandServiceExt for NatsStore {
    #[instrument(skip_all, level = "debug")]
    async fn spawn_service<A>(&self) -> error::Result<()>
    where
        A: Aggregate + 'static,
        A::Command: serde::de::DeserializeOwned,
    {
        let mut store = self.clone();
        let tracker = self.graceful_shutdown.task_tracker.clone();
        let exit_tx = self.graceful_shutdown.exit_tx.clone();

        // Obtain a stream_cancel pair so the spawned task can be cancelled
        // during graceful shutdown. The Trigger is sent to the shutdown
        // receiver; dropping it signals cancellation to the Valve.
        let (trigger, valve) = stream_cancel::Tripwire::new();

        // Register the trigger with the graceful shutdown channel so
        // wait_graceful_shutdown() will cancel this task when called.
        exit_tx
            .send(trigger)
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        tracker.spawn(async move {
            // Wrap serve so that the task exits when the valve fires.
            let serve = store.serve::<A>();
            tokio::select! {
                result = serve => {
                    if let Err(e) = result {
                        tracing::error!("command service exited with error: {e}");
                    }
                }
                _ = valve => {
                    tracing::debug!("command service task received shutdown signal");
                }
            }
        });

        Ok(())
    }
}
