use async_nats::jetstream::Message;
use futures::{Stream, StreamExt, TryStreamExt};
use stream_cancel::Valved;
use tracing::instrument;

use crate::{error, nats::NatsStore, EventGroup};

impl NatsStore {
    #[instrument(skip_all, level = "debug")]
    async fn legacy_subscribe(
        &self,
        unique_name: &str,
        subjects: Vec<impl Into<String> + Send + Sync>,
    ) -> error::Result<impl Stream<Item = error::Result<Message>> + Send> {
        let subjects: Vec<String> = subjects.into_iter().map(|s| s.into()).collect();

        let consumer = self
            .durable_consumer(unique_name.to_string(), subjects)
            .await?;
        let messages = consumer
            .messages()
            .await?
            .map_err(|e| error::Error::Format(e.into()));
        Ok(messages)
    }

    /// subscribe to the given subjects, and process incoming messages with the given projector.
    #[instrument(skip_all, level = "debug")]
    pub async fn run_legacy_project<P>(
        &self,
        projector: P,
        feature_name: &str,
        subjects: Vec<impl Into<String> + Send + Sync>,
    ) -> error::Result<()>
    where
        P: LegacyProject + 'static,
    {
        let stream = std::pin::pin!(self.legacy_subscribe(feature_name, subjects).await?);
        let (exit, mut incoming) = Valved::new(stream);
        self.graceful_shutdown
            .exit_tx
            .clone()
            .send(exit)
            .await
            .expect("should be able to send graceful trigger");

        while let Some(message) = incoming.next().await {
            let mut projector = projector.clone();

            let _ = process_legacy_message(&mut projector, message).await;
        }

        Ok(())
    }
}

/// A data model that can be "projected" onto.
///
/// That is, receive events for all aggregate IDs for an event or events, and
/// process them to trigger a side effect or construct a read model. The exact
/// purpose is implementation specific; the trait only handles receiving the
/// Envelopes for the specified events.
#[trait_variant::make(Send)]
pub trait LegacyProject: Send + Clone {
    /// The event(s) that can be processed by this object.
    type EventGroup: EventGroup + serde::de::DeserializeOwned + Send;
    /// The type to return as an `Err` when the projection fails.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Apply a received event, triggering implementation specific behavior.
    ///
    /// Returning an error from this method should stop further messages from
    /// being processed in the associated event store.
    async fn project(&mut self, event: Self::EventGroup) -> Result<(), Self::Error>;
}

/// recieves a message, processes it with the given projector, and acknowledges it.
#[instrument(skip_all, name = "legacy_automation", level = "info", fields(aggregate=tracing::field::Empty) err)]
async fn process_legacy_message<P: LegacyProject>(
    projector: &mut P,
    message: Result<Message, error::Error>,
) -> error::Result<()> {
    let envelope = message?;
    // propagate otel span if exists
    opentelemetry_nats::attach_span_context(&envelope);

    let event: P::EventGroup = serde_json::from_slice(&envelope.payload).unwrap();

    projector
        .project(event)
        .await
        .map_err(|e| error::Error::External(e.into()))?;
    let _ = envelope.ack().await;
    Ok(())
}
