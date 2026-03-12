====
>   Info: Duration: 2m 22s | ~$0.1304 | Prompt Tokens: 29,858 | Completion Tokens: 9,312 (reasoning: 5,440) 
>  Model: gpt-5 (gpt-5-2025-08-07) | Adapter: OpenAI
>  Files: Context Files: 44 | Working Files: 0 | Knowledge Files: 0

````rs
// file: src/nats.rs
use std::sync::{Arc, Mutex};

use async_nats::jetstream::consumer::pull::{Config as ConsumerConfig, OrderedConfig};
use async_nats::jetstream::consumer::{Consumer, DeliverPolicy};
use async_nats::jetstream::stream::{
    Config as StreamConfig, DiscardPolicy, Mirror as StreamMirror, Stream as JetStream,
};
use async_nats::jetstream::Context;
use stream_cancel::Trigger;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::task::TaskTracker;
use tracing::instrument;

use crate::error;

#[doc(hidden)]
pub mod convert;
/// Use a Jetstream message as an esrc Envelope.
pub mod envelope;
#[doc(hidden)]
pub mod event;

/// Legacy support for older projects that needed the automation control of the NatsStore.
pub mod legacy;

/// Dead letter queue functionality for handling undelivered messages.
pub mod dead_letter;

pub use dead_letter::{DeadLetterMessage, DeadLetterReason, DeadLetterStore};
pub use envelope::NatsEnvelope;

mod header;
mod subject;

use subject::NatsSubject;

/// A handle to an event store implementation on top of NATS.
///
/// This type implements the needed traits for reading and writing events from
/// various event streams, encoded as durable messages in a Jetstream instance.
#[derive(Clone)]
pub struct NatsStore {
    prefix: &'static str,

    context: Context,
    stream: JetStream,

    // When set, all read consumers will be created against this mirror stream
    // instead of the default `stream`. This allows isolating read state per
    // feature while keeping writes in the original stream.
    mirror_stream: Option<JetStream>,

    graceful_shutdown: GracefulShutdown,

    durable_consumer_options: ConsumerConfig,
}

/// A structure to help with graceful shutdown of tasks.
#[derive(Clone)]
pub struct GracefulShutdown {
    task_tracker: TaskTracker,
    exit_rx: Arc<Mutex<Receiver<Trigger>>>,
    exit_tx: Sender<Trigger>,
}

impl NatsStore {
    /// Create a new instance of a NATS event store.
    ///
    /// This uses an existing Jetstream context and a global prefix string. The
    /// method will attempt to use an existing stream with this name, or create
    /// a new one with default settings. All esrc event streams are created with
    /// this prefix, using the format `<prefix>.<event_name>.<aggregate_id>`.
    #[instrument(skip_all, level = "debug")]
    pub async fn try_new(context: Context, prefix: &'static str) -> error::Result<Self> {
        let stream = {
            let config = StreamConfig {
                name: prefix.to_owned(),
                subjects: vec![NatsSubject::Wildcard.into_string(prefix)],
                discard: DiscardPolicy::New,
                ..Default::default()
            };

            context.get_or_create_stream(config).await?
        };

        let config = ConsumerConfig {
            deliver_policy: DeliverPolicy::New,
            ..Default::default()
        };

        // if there is more than 1000 automations this should be increased
        let (exit_tx, exit_rx) = tokio::sync::mpsc::channel::<stream_cancel::Trigger>(1000);
        let task_tracker = tokio_util::task::TaskTracker::new();

        let graceful_shutdown = GracefulShutdown {
            exit_tx,
            exit_rx: Mutex::new(exit_rx).into(),
            task_tracker,
        };

        Ok(Self {
            prefix,

            context,
            stream,

            mirror_stream: None,

            graceful_shutdown,

            durable_consumer_options: config,
        })
    }

    /// Enable reading from a mirror stream instead of the default stream.
    ///
    /// The mirror will be created (or reused if it exists) and will mirror the
    /// entire writer stream identified by `prefix`. Consumers created for
    /// replay/subscribe APIs will be attached to this mirror stream.
    #[instrument(skip_all, level = "debug")]
    pub async fn enable_mirror(mut self, mirror_name: impl Into<String>) -> error::Result<Self> {
        let mirror_name = mirror_name.into();

        let config = StreamConfig {
            name: mirror_name,
            // Mirror the writer stream (self.prefix). Filtering remains at the consumer level.
            mirror: Some(StreamMirror {
                name: self.prefix.to_owned(),
                ..Default::default()
            }),
            discard: DiscardPolicy::New,
            ..Default::default()
        };

        let mirror_stream = self.context.get_or_create_stream(config).await?;
        self.mirror_stream = Some(mirror_stream);

        Ok(self)
    }

    /// get a handle to the task tracker used for graceful shutdown of tasks
    pub fn get_task_tracker(&self) -> TaskTracker {
        self.graceful_shutdown.task_tracker.clone()
    }

    ///
    pub async fn wait_graceful_shutdown(self) {
        let mut exit_rx = self
            .graceful_shutdown
            .exit_rx
            .lock()
            .expect("lock to not be poisoned");
        while let Some(trigger) = exit_rx.try_recv().ok() {
            println!("triggering graceful shutdown");
            trigger.cancel();
        }
        self.graceful_shutdown.task_tracker.close();
        self.graceful_shutdown.task_tracker.wait().await;
    }

    /// the subjects and durable name of the consumer are overwritten by the function that starts
    /// the consumer, all other options should be alright for modification
    pub fn update_durable_consumer_option(mut self, options: ConsumerConfig) -> Self {
        self.durable_consumer_options = options;
        self
    }

    /// Select the stream used for creating read-side consumers.
    fn reader_stream(&self) -> &JetStream {
        self.mirror_stream.as_ref().unwrap_or(&self.stream)
    }

    #[instrument(skip_all, level = "debug")]
    async fn ordered_consumer(
        &self,
        subjects: Vec<String>,
        start_sequence: u64,
    ) -> error::Result<Consumer<OrderedConfig>> {
        let mut config = OrderedConfig {
            filter_subjects: subjects,
            ..Default::default()
        };

        if start_sequence > 0 {
            config.deliver_policy = DeliverPolicy::ByStartSequence { start_sequence };
        }

        Ok(self.reader_stream().create_consumer(config).await?)
    }

    #[instrument(skip_all, level = "debug")]
    async fn durable_consumer(
        &self,
        name: String,
        subjects: Vec<String>,
    ) -> error::Result<Consumer<ConsumerConfig>> {
        let mut config = self.durable_consumer_options.clone();

        config.filter_subjects = subjects;
        config.durable_name = Some(name);

        Ok(self.reader_stream().create_consumer(config).await?)
    }

    // Internal helper for cases that must always attach to the writer stream.
    #[instrument(skip_all, level = "debug")]
    async fn durable_consumer_on_writer(
        &self,
        name: String,
        subjects: Vec<String>,
    ) -> error::Result<Consumer<ConsumerConfig>> {
        let mut config = self.durable_consumer_options.clone();

        config.filter_subjects = subjects;
        config.durable_name = Some(name);

        Ok(self.stream.create_consumer(config).await?)
    }
}

````


````rs
// file: src/nats/dead_letter.rs
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

        // Always attach this consumer to the writer stream; advisories are not mirrored.
        let advisory_consumer = self
            .durable_consumer_on_writer(durable_name.to_string(), subjects)
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

````

