use std::sync::{Arc, Mutex};

use async_nats::jetstream::consumer::pull::{Config as ConsumerConfig, OrderedConfig};
use async_nats::jetstream::consumer::{Consumer, DeliverPolicy};
use async_nats::jetstream::stream::{
    Config as StreamConfig, DiscardPolicy, Source as StreamMirror, Stream as JetStream,
};
use async_nats::jetstream::Context;
use futures::StreamExt;
use stream_cancel::{Trigger, Tripwire};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::task::TaskTracker;
use tracing::error;
use tracing::instrument;

use crate::error;
use crate::event_modeling::{Automation, ConsumerSpec, ExecutionPolicy, ReadModel};
use crate::project::Project;
use crate::query::{Query, QueryHandler, QueryService};

/// NATS-backed command service support for `NatsStore`.
pub mod command_service;
#[doc(hidden)]
pub mod convert;
/// Use a Jetstream message as an esrc Envelope.
pub mod envelope;
#[doc(hidden)]
pub mod event;
/// NATS-backed query service support for `NatsStore`.
pub mod query_service;
/// NATS JetStream Key-Value backed QueryHandler implementation.
pub mod query_kv;

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

    /// return a clone of the underlying Nats Client
    pub fn client(&self) -> async_nats::Client {
        self.context.client()
    }

    /// Returns the underlying JetStream context.
    ///
    /// This can be used to create Key-Value stores or perform other
    /// JetStream operations outside the event store abstraction.
    pub fn jetstream_context(&self) -> &Context {
        &self.context
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
        let mut config = ConsumerConfig {
            deliver_policy: DeliverPolicy::New,
            ..Default::default()
        };

        config.filter_subjects = subjects;
        config.durable_name = Some(name);

        Ok(self.stream.create_consumer(config).await?)
    }

    /// Run a consumer with the given specification.
    #[instrument(skip_all, level = "debug")]
    pub async fn run_consumer<P>(&self, spec: ConsumerSpec<P>) -> error::Result<()>
    where
        P: Project + Send + Sync + Clone + 'static,
        P::EventGroup: crate::event::EventGroup + Send,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let durable_name = spec.name().durable_name();
        let mut names = <P::EventGroup as crate::event::EventGroup>::names().collect::<Vec<_>>();
        names.sort();

        let subjects = names
            .iter()
            .map(|&n| NatsSubject::Event(n.into()).into_string(self.prefix))
            .collect();

        let consumer = self.durable_consumer(durable_name, subjects).await?;
        let stream = consumer
            .messages()
            .await?
            .map(|m| NatsEnvelope::try_from_message(self.prefix, m?));

        match spec.execution_policy() {
            ExecutionPolicy::Sequential => self.run_consumer_sequential(spec, stream).await,
            ExecutionPolicy::Concurrent { max_in_flight } => {
                if max_in_flight == 0 {
                    return Err(error::Error::Configuration(
                        "concurrent consumers require max_in_flight > 0".to_owned(),
                    ));
                }

                self.run_consumer_concurrent(spec, stream, max_in_flight)
                    .await
            },
        }
    }

    /// Spawn a declared consumer onto the store task tracker.
    ///
    /// This keeps runtime ownership of task lifecycle and background error
    /// reporting inside infrastructure while letting startup code stay concise.
    pub fn spawn_consumer<P>(&self, spec: ConsumerSpec<P>)
    where
        P: Project + Send + Sync + Clone + 'static,
        P::EventGroup: crate::event::EventGroup + Send,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let store = self.clone();
        let durable_name = spec.name().durable_name();
        let (trigger, tripwire) = Tripwire::new();

        let exit_tx = self.graceful_shutdown.exit_tx.clone();

        self.graceful_shutdown.task_tracker.spawn(async move {
            // Register the trigger so it is cancelled during graceful shutdown.
            if exit_tx.send(trigger).await.is_err() {
                tracing::warn!("failed to register shutdown trigger for command service");
                return;
            }

            tokio::select! {
                result = store.run_consumer(spec) => {
                    if let Err(e) = result {
                        error!(consumer = %durable_name, err=?e, "consumer stopped");
                    }
                }
                _ = tripwire => {
                    tracing::info!(consumer = %durable_name, "shutting down consumer");
                }
            }
        });
    }

    /// Spawn an automation declaration onto the store task tracker.
    pub fn spawn_automation<P>(&self, automation: Automation<P>)
    where
        P: Project + Send + Sync + Clone + 'static,
        P::EventGroup: crate::event::EventGroup + Send,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        self.spawn_consumer(automation.into_spec());
    }

    /// Spawn a read model declaration onto the store task tracker.
    pub fn spawn_read_model<P>(&self, read_model: ReadModel<P>)
    where
        P: Project + Send + Sync + Clone + 'static,
        P::EventGroup: crate::event::EventGroup + Send,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        self.spawn_consumer(read_model.into_spec());
    }

    /// Spawn a read model slice, starting both the event consumer and the
    /// query service as tracked background tasks.
    ///
    /// This is the recommended way to register a vertical slice that pairs
    /// a read model projection with its query handler.
    pub fn spawn_read_model_slice<P, H>(
        &self,
        slice: crate::event_modeling::ReadModelSlice<P, H>,
    ) where
        P: Project + Send + Sync + Clone + 'static,
        P::EventGroup: crate::event::EventGroup + Send,
        P::Error: std::error::Error + Send + Sync + 'static,
        H: QueryHandler + Send + Sync + 'static,
        H::Query: serde::de::DeserializeOwned + Sync,
        H::Id: serde::de::DeserializeOwned,
        <H::Query as Query>::ReadModel: serde::Serialize + Sync,
        <H::Query as Query>::Response: serde::Serialize + Sync,
    {
        let (consumer_spec, query_spec) = slice.into_specs();

        // Spawn the event consumer (read model projection).
        self.spawn_consumer(consumer_spec);

        // Spawn the query service.
        self.spawn_query_service(query_spec);
    }
}
