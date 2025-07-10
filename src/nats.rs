use async_nats::jetstream::consumer::pull::{Config as ConsumerConfig, OrderedConfig};
use async_nats::jetstream::consumer::{Consumer, DeliverPolicy};
use async_nats::jetstream::stream::{Config as StreamConfig, DiscardPolicy, Stream as JetStream};
use async_nats::jetstream::Context;
use tracing::instrument;

use crate::error;

#[doc(hidden)]
pub mod convert;
/// Use a Jetstream message as an esrc Envelope.
pub mod envelope;
#[doc(hidden)]
pub mod event;

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

        Ok(Self {
            prefix,

            context,
            stream,
        })
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

        Ok(self.stream.create_consumer(config).await?)
    }

    #[instrument(skip_all, level = "debug")]
    async fn durable_consumer(
        &self,
        name: String,
        subjects: Vec<String>,
    ) -> error::Result<Consumer<ConsumerConfig>> {
        let config = ConsumerConfig {
            filter_subjects: subjects,
            durable_name: Some(name),
            deliver_policy: DeliverPolicy::New,
            ..Default::default()
        };

        Ok(self.stream.create_consumer(config).await?)
    }
}
