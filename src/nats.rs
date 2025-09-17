use std::sync::{Arc, Mutex};

use async_nats::jetstream::consumer::pull::{Config as ConsumerConfig, OrderedConfig};
use async_nats::jetstream::consumer::{Consumer, DeliverPolicy};
use async_nats::jetstream::stream::{Config as StreamConfig, DiscardPolicy, Stream as JetStream};
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

            graceful_shutdown,

            durable_consumer_options: config,
        })
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
        let mut config = self.durable_consumer_options.clone();

        config.filter_subjects = subjects;
        config.durable_name = Some(name);

        Ok(self.stream.create_consumer(config).await?)
    }
}
