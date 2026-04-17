use async_nats::jetstream::Message;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use time::OffsetDateTime;
use tracing::{error, info, warn};

/// Represents a dead letter message with metadata about the failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterMessage {
    /// The original message subject
    pub subject: String,
    /// The original message payload
    pub payload: Vec<u8>,
    /// The original message headers, if any
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// The stream name where the message originated
    pub stream: String,
    /// The consumer name that failed to process the message
    pub consumer: String,
    /// The sequence number of the message in the stream
    pub stream_sequence: u64,
    /// The reason for the message being dead lettered
    pub reason: DeadLetterReason,
    /// When the message was dead lettered
    pub timestamp: OffsetDateTime,
    /// Number of delivery attempts before being dead lettered
    pub delivery_count: u64,
}

/// The reason why a message was added to the dead letter queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeadLetterReason {
    /// Message reached maximum delivery attempts
    MaxDeliveriesReached,
    /// Message was manually terminated by client
    MessageTerminated,
    /// Processing error occurred
    ProcessingError(String),
}

/// Extension trait for storing dead letter messages
///
/// This trait allows implementations to use any storage backend
/// (database, file system, another NATS stream, etc.)
#[trait_variant::make(Send)]
pub trait DeadLetterStore: Send + Sync {
    /// Error type for storage operations
    type Error: StdError + Send + Sync + 'static;

    /// Store a dead letter message
    async fn store_dead_letter(&self, message: DeadLetterMessage) -> Result<(), Self::Error>;

    /// Retrieve dead letter messages with optional filtering
    async fn get_dead_letters(
        &self,
        stream: Option<&str>,
        consumer: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<DeadLetterMessage>, Self::Error>;

    /// Remove a dead letter message by its unique identifier
    /// The identifier is implementation-specific (could be sequence, ID, etc.)
    async fn remove_dead_letter(&self, identifier: &str) -> Result<(), Self::Error>;

    /// Get count of dead letter messages with optional filtering
    async fn count_dead_letters(
        &self,
        stream: Option<&str>,
        consumer: Option<&str>,
    ) -> Result<u64, Self::Error>;
}

impl DeadLetterMessage {
    /// Create a new dead letter message from a NATS message and advisory info
    pub fn new(
        message: &Message,
        stream: String,
        consumer: String,
        stream_sequence: u64,
        reason: DeadLetterReason,
        delivery_count: u64,
    ) -> Self {
        let headers = message.headers.as_ref().map(|h| {
            h.iter()
                .map(|(k, v)| {
                    (
                        k.to_string(),
                        v.iter()
                            .map(|val| val.to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                    )
                })
                .collect()
        });

        // Extract timestamp from message info, fallback to current time if unavailable
        let timestamp = message
            .info()
            .map(|info| info.published)
            .unwrap_or_else(|_| OffsetDateTime::now_utc());

        Self {
            subject: message.subject.to_string(),
            payload: message.payload.to_vec(),
            headers,
            stream,
            consumer,
            stream_sequence,
            reason,
            timestamp,
            delivery_count,
        }
    }

    /// Create a new dead letter message from a NATS stream message and advisory info
    pub fn from_stream_message(
        message: &async_nats::jetstream::message::StreamMessage,
        stream: String,
        consumer: String,
        stream_sequence: u64,
        reason: DeadLetterReason,
        delivery_count: u64,
    ) -> Self {
        let headers = if message.headers.is_empty() {
            None
        } else {
            Some(
                message
                    .headers
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.to_string(),
                            v.iter()
                                .map(|val| val.to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                        )
                    })
                    .collect(),
            )
        };

        // Extract timestamp from message time
        let timestamp = message.time;

        Self {
            subject: message.subject.to_string(),
            payload: message.payload.to_vec(),
            headers,
            stream,
            consumer,
            stream_sequence,
            reason,
            timestamp,
            delivery_count,
        }
    }

    /// Create a dead letter message from advisory message data
    pub fn from_advisory(
        stream: String,
        consumer: String,
        stream_sequence: u64,
        reason: DeadLetterReason,
        delivery_count: u64,
        original_subject: String,
        payload: Vec<u8>,
        headers: Option<std::collections::HashMap<String, String>>,
    ) -> Self {
        Self {
            subject: original_subject,
            payload,
            headers,
            stream,
            consumer,
            stream_sequence,
            reason,
            timestamp: OffsetDateTime::now_utc(),
            delivery_count,
        }
    }
}

impl crate::nats::NatsStore {
    /// Start dead letter queue automation for a specific stream and consumer
    ///
    /// This method listens to NATS JetStream advisory messages for max deliveries
    /// and terminated messages, then stores the original messages using the provided
    /// dead letter store implementation.
    pub async fn run_dead_letter_automation<D>(
        &self,
        dead_letter_store: D,
        durable_name: &str,
        stream_name: &str,
        consumer_name: &str,
    ) -> crate::error::Result<()>
    where
        D: DeadLetterStore + Clone + 'static,
    {
        let max_deliveries_subject = format!(
            "$JS.EVENT.ADVISORY.CONSUMER.MAX_DELIVERIES.{}.{}",
            stream_name, consumer_name
        );
        let terminated_subject = format!(
            "$JS.EVENT.ADVISORY.CONSUMER.MSG_TERMINATED.{}.{}",
            stream_name, consumer_name
        );

        let subjects = vec![max_deliveries_subject, terminated_subject];

        // Create a consumer for advisory messages
        let advisory_consumer = self
            .durable_consumer(durable_name.to_string(), subjects)
            .await?;

        let messages = advisory_consumer
            .messages()
            .await?
            .map_err(|e| crate::error::Error::Format(e.into()));

        let (exit, incoming) = stream_cancel::Valved::new(messages);
        self.graceful_shutdown
            .exit_tx
            .clone()
            .send(exit)
            .await
            .expect("should be able to send graceful trigger");

        let store = dead_letter_store.clone();
        let stream_name = stream_name.to_string();
        let consumer_name = consumer_name.to_string();
        let nats_stream = self.stream.clone();

        self.graceful_shutdown.task_tracker.spawn(async move {
            let mut incoming = incoming;
            while let Some(advisory_msg_result) = incoming.next().await {
                match advisory_msg_result {
                    Ok(advisory_msg) => {
                        if let Err(e) = process_advisory_message(
                            &store,
                            &nats_stream,
                            &stream_name,
                            &consumer_name,
                            advisory_msg,
                        )
                        .await
                        {
                            tracing::error!("Error processing dead letter advisory: {:?}", e);
                        }
                    },
                    Err(e) => {
                        tracing::error!("Error receiving advisory message: {:?}", e);
                    },
                }
            }
        });

        Ok(())
    }
}

async fn process_advisory_message<D>(
    store: &D,
    nats_stream: &async_nats::jetstream::stream::Stream,
    stream_name: &str,
    consumer_name: &str,
    advisory_msg: async_nats::jetstream::Message,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    D: DeadLetterStore,
{
    let subject = advisory_msg.subject.to_string();
    let payload = &advisory_msg.payload;

    // Parse the advisory message to get the stream sequence
    let advisory_data: serde_json::Value = serde_json::from_slice(payload)?;

    let stream_seq = advisory_data
        .get("stream_seq")
        .and_then(|v| v.as_u64())
        .ok_or("Missing stream_seq in advisory message")?;

    let delivery_count = advisory_data
        .get("deliveries")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Determine the reason based on the advisory subject
    let reason = if subject.contains("MAX_DELIVERIES") {
        DeadLetterReason::MaxDeliveriesReached
    } else if subject.contains("MSG_TERMINATED") {
        DeadLetterReason::MessageTerminated
    } else {
        DeadLetterReason::ProcessingError("Unknown advisory type".to_string())
    };

    // Retrieve the original message by sequence number
    match nats_stream.get_raw_message(stream_seq).await {
        Ok(original_msg) => {
            let dead_letter_msg = DeadLetterMessage::from_stream_message(
                &original_msg,
                stream_name.to_string(),
                consumer_name.to_string(),
                stream_seq,
                reason,
                delivery_count,
            );

            if let Err(e) = store.store_dead_letter(dead_letter_msg).await {
                error!("Failed to store dead letter message: {:?}", e);
            } else {
                info!(
                    "Stored dead letter message: stream={}, consumer={}, seq={}",
                    stream_name, consumer_name, stream_seq
                );
            }
        },
        Err(e) => {
            warn!(
                "Could not retrieve original message for sequence {}: {:?}",
                stream_seq, e
            );

            // Create a dead letter message with just the advisory information
            let dead_letter_msg = DeadLetterMessage::from_advisory(
                stream_name.to_string(),
                consumer_name.to_string(),
                stream_seq,
                reason,
                delivery_count,
                "unknown".to_string(),
                vec![],
                None,
            );

            if let Err(e) = store.store_dead_letter(dead_letter_msg).await {
                error!("Failed to store dead letter message from advisory: {:?}", e);
            }
        },
    }

    // Acknowledge the advisory message
    let _ = advisory_msg.ack().await;

    Ok(())
}
