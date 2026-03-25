use async_nats::service::ServiceExt;
use futures::StreamExt;
use tracing::instrument;

use crate::aggregate::Aggregate;
use crate::error::{self, Error};
use crate::event::command_service::{CommandError, CommandErrorKind, CommandService};
use crate::event::publish::PublishExt;
use crate::event::replay::ReplayOneExt;
use crate::event::{Event, Sequence};
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
            .endpoint("command")
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
                }
            };

            // Deserialize the command from the request payload.
            let command: A::Command =
                match serde_json::from_slice(&request.message.payload) {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        let err = CommandError::new(
                            CommandErrorKind::InvalidPayload,
                            format!("failed to deserialize command: {e}"),
                        );
                        reply_error(&request, err).await;
                        continue;
                    }
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
                }
            };

            // Process the command and publish the resulting event.
            let mut store = self.clone();
            match store.try_write(root, command, None).await {
                Ok(_) => {
                    if let Err(e) = request.respond(Ok(bytes::Bytes::new())).await {
                        tracing::warn!("failed to send success reply: {e}");
                    }
                }
                Err(Error::Conflict) => {
                    let err = CommandError::new(
                        CommandErrorKind::Conflict,
                        "optimistic concurrency conflict",
                    );
                    reply_error(&request, err).await;
                }
                Err(Error::External(e)) => {
                    let err = CommandError::new(
                        CommandErrorKind::Domain,
                        format!("domain error: {e}"),
                    );
                    reply_error(&request, err).await;
                }
                Err(e) => {
                    let err = CommandError::new(
                        CommandErrorKind::Internal,
                        format!("internal error: {e}"),
                    );
                    reply_error(&request, err).await;
                }
            }
        }

        Ok(())
    }
}

async fn reply_error(request: &async_nats::service::Request, error: CommandError) {
    let payload = serde_json::to_vec(&error).unwrap_or_default();
    if let Err(e) = request
        .respond(Err(async_nats::service::error::Error {
            status: error.status_code(),
            description: error.message.clone(),
        }))
        .await
    {
        tracing::warn!("failed to send error reply: {e}");
    }
    // The NATS service error response does not carry a custom body,
    // so we log the structured payload for observability.
    tracing::debug!(
        error_kind = ?error.kind,
        error_message = %error.message,
        "command service replied with error"
    );
}
