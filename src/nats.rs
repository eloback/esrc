use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_nats::jetstream::consumer::pull::{Config as ConsumerConfig, OrderedConfig};
use async_nats::jetstream::consumer::{
    AckPolicy, Config as BaseConsumerConfig, Consumer, DeliverPolicy,
};
use async_nats::jetstream::stream::{
    Config as StreamConfig, DiscardPolicy, Source as StreamMirror, Stream as JetStream,
};
use async_nats::jetstream::Context;
use stream_cancel::Trigger;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::task::TaskTracker;
use tracing::instrument;

use crate::error;
use crate::event::event_model::ViewProjectorIdentity;

/// The NATS client version used by this event-store backend.
pub use async_nats;

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

const DEFAULT_ACK_WAIT: Duration = Duration::from_secs(30);
const VIEW_PROJECTOR_ID_METADATA_KEY: &str = "esrc-view-projector-id";
const VIEW_PROJECTOR_VERSION_METADATA_KEY: &str = "esrc-view-projector-version";

/// Supported replication policies for a NATS event stream.
///
/// A single replica preserves the historical behavior of [`NatsStore::try_new`].
/// Three replicas are intended for a three-server JetStream cluster and allow
/// the stream to retain quorum when one server is unavailable.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NatsStreamReplicas {
    /// Store one copy of each event.
    #[default]
    One,
    /// Store three copies of each event.
    Three,
}

impl NatsStreamReplicas {
    /// Return the replica count sent to JetStream.
    pub const fn count(self) -> usize {
        match self {
            Self::One => 1,
            Self::Three => 3,
        }
    }
}

/// Configuration used when opening or creating a [`NatsStore`].
///
/// The default retains the historical one-replica behavior. Applications using
/// a three-server production cluster should opt in with [`Self::replicated`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NatsStoreOptions {
    stream_replicas: NatsStreamReplicas,
}

impl NatsStoreOptions {
    /// Create options that request three replicas for writer and mirror streams.
    pub const fn replicated() -> Self {
        Self {
            stream_replicas: NatsStreamReplicas::Three,
        }
    }

    /// Return the requested stream replication policy.
    pub const fn stream_replicas(self) -> NatsStreamReplicas {
        self.stream_replicas
    }
}

/// The requested replica policy does not match an existing stream.
///
/// The framework does not mutate the existing authoritative stream
/// automatically. Operators can inspect this error and perform a separately
/// controlled replica migration.
#[derive(Debug, thiserror::Error)]
#[error("NATS stream `{stream}` has {actual} replicas, but the application requires {expected}")]
pub struct NatsStreamReplicaMismatch {
    stream: String,
    expected: usize,
    actual: usize,
}

/// A durable view consumer is assigned to another logical projector identity or version.
#[derive(Debug, thiserror::Error)]
#[error("NATS view consumer `{durable}` is assigned to projector `{actual}`, not `{expected}`")]
pub struct NatsViewConsumerIdentityMismatch {
    durable: String,
    expected: String,
    actual: String,
}

impl NatsViewConsumerIdentityMismatch {
    /// Return the durable consumer name that had the conflicting assignment.
    pub fn durable(&self) -> &str {
        &self.durable
    }

    /// Return the projector identity required by this application instance.
    pub fn expected(&self) -> &str {
        &self.expected
    }

    /// Return the projector identity stored on the durable consumer.
    pub fn actual(&self) -> &str {
        &self.actual
    }
}

impl NatsStreamReplicaMismatch {
    /// Return the affected stream name.
    pub fn stream(&self) -> &str {
        &self.stream
    }

    /// Return the replica count requested by the application.
    pub const fn expected(&self) -> usize {
        self.expected
    }

    /// Return the replica count reported by the existing stream.
    pub const fn actual(&self) -> usize {
        self.actual
    }
}

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

    options: NatsStoreOptions,

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
    /// a new one with one replica. All esrc event streams are created with
    /// this prefix, using the format `<prefix>.<event_name>.<aggregate_id>`.
    ///
    /// Use [`Self::try_new_with_options`] with [`NatsStoreOptions::replicated`]
    /// when the application requires three replicas.
    #[instrument(skip_all, level = "debug")]
    pub async fn try_new(context: Context, prefix: &'static str) -> error::Result<Self> {
        Self::try_new_with_options(context, prefix, NatsStoreOptions::default()).await
    }

    /// Create a NATS event store with an explicit stream replication policy.
    ///
    /// New writer streams use the requested replica count. If a stream already
    /// exists with another count, this returns a [`NatsStreamReplicaMismatch`]
    /// wrapped in [`crate::Error::Internal`] instead of mutating the stream.
    #[instrument(skip_all, level = "debug")]
    pub async fn try_new_with_options(
        context: Context,
        prefix: &'static str,
        options: NatsStoreOptions,
    ) -> error::Result<Self> {
        let config = StreamConfig {
            name: prefix.to_owned(),
            subjects: vec![NatsSubject::Wildcard.into_string(prefix)],
            discard: DiscardPolicy::New,
            num_replicas: options.stream_replicas().count(),
            ..Default::default()
        };
        let stream = get_or_create_validated_stream(&context, config).await?;

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

            options,

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
            num_replicas: self.options.stream_replicas().count(),
            ..Default::default()
        };

        let mirror_stream = get_or_create_validated_stream(&self.context, config).await?;
        self.mirror_stream = Some(mirror_stream);

        Ok(self)
    }

    /// get a handle to the task tracker used for graceful shutdown of tasks
    pub fn get_task_tracker(&self) -> TaskTracker {
        self.graceful_shutdown.task_tracker.clone()
    }

    /// Cancel registered automation streams and wait for their tracked tasks to finish.
    pub async fn wait_graceful_shutdown(self) {
        {
            let mut exit_rx = self
                .graceful_shutdown
                .exit_rx
                .lock()
                .expect("lock to not be poisoned");
            while let Ok(trigger) = exit_rx.try_recv() {
                println!("triggering graceful shutdown");
                trigger.cancel();
            }
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
        let config = durable_consumer_config(self.durable_consumer_options.clone(), name, subjects);

        Ok(self.reader_stream().create_consumer(config).await?)
    }

    #[instrument(skip_all, level = "debug")]
    async fn view_durable_consumer(
        &self,
        name: String,
        subjects: Vec<String>,
        identity: &ViewProjectorIdentity,
    ) -> error::Result<Consumer<ConsumerConfig>> {
        let config = view_consumer_config(
            self.durable_consumer_options.clone(),
            name.clone(),
            subjects,
            identity,
        );
        let stream = self.reader_stream();
        let existing = stream.get_or_create_consumer(&name, config.clone()).await?;
        validate_view_consumer(&name, &existing.cached_info().config, &config, identity)?;

        // Preserve the historical update behavior after validating the stable logical identity.
        let consumer = stream.create_consumer(config.clone()).await?;
        validate_view_consumer(&name, &consumer.cached_info().config, &config, identity)?;

        Ok(consumer)
    }
}

fn durable_consumer_config(
    mut config: ConsumerConfig,
    name: String,
    subjects: Vec<String>,
) -> ConsumerConfig {
    config.filter_subjects = subjects;
    config.durable_name = Some(name);
    config
}

fn view_consumer_config(
    config: ConsumerConfig,
    name: String,
    subjects: Vec<String>,
    projector_identity: &ViewProjectorIdentity,
) -> ConsumerConfig {
    let mut config = durable_consumer_config(config, name, subjects);
    config.ack_policy = AckPolicy::Explicit;
    config.max_ack_pending = 1;
    config.max_deliver = -1;
    if config.ack_wait.is_zero() {
        config.ack_wait = DEFAULT_ACK_WAIT;
    }
    config.metadata.insert(
        VIEW_PROJECTOR_ID_METADATA_KEY.to_owned(),
        projector_identity.id().to_owned(),
    );
    config.metadata.insert(
        VIEW_PROJECTOR_VERSION_METADATA_KEY.to_owned(),
        projector_identity.version().to_string(),
    );
    config
}

fn validate_view_consumer(
    durable: &str,
    existing: &BaseConsumerConfig,
    expected_config: &ConsumerConfig,
    expected_identity: &ViewProjectorIdentity,
) -> error::Result<()> {
    let expected = identity_label(expected_identity);
    let expected_version = expected_identity.version().to_string();
    let actual_id = existing.metadata.get(VIEW_PROJECTOR_ID_METADATA_KEY);
    let actual_version = existing.metadata.get(VIEW_PROJECTOR_VERSION_METADATA_KEY);
    if actual_id.map(String::as_str) != Some(expected_identity.id())
        || actual_version.map(String::as_str) != Some(expected_version.as_str())
    {
        let actual = match (actual_id, actual_version) {
            (Some(id), Some(version)) => format!("{id}@{version}"),
            _ => "<missing stable projector identity>".to_owned(),
        };
        return Err(view_identity_mismatch(durable, &expected, actual));
    }
    if normalized_base_filters(existing) != normalized_filters(expected_config) {
        return Err(view_identity_mismatch(
            durable,
            &expected,
            format!("{expected} with different filters"),
        ));
    }
    Ok(())
}

fn identity_label(identity: &ViewProjectorIdentity) -> String {
    format!("{}@{}", identity.id(), identity.version())
}

fn normalized_filters(config: &ConsumerConfig) -> Vec<&str> {
    let mut filters = config
        .filter_subjects
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !config.filter_subject.is_empty() {
        filters.push(&config.filter_subject);
    }
    filters.sort_unstable();
    filters
}

fn normalized_base_filters(config: &BaseConsumerConfig) -> Vec<&str> {
    let mut filters = config
        .filter_subjects
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !config.filter_subject.is_empty() {
        filters.push(&config.filter_subject);
    }
    filters.sort_unstable();
    filters
}

fn view_identity_mismatch(durable: &str, expected: &str, actual: String) -> error::Error {
    error::Error::Internal(Box::new(NatsViewConsumerIdentityMismatch {
        durable: durable.to_owned(),
        expected: expected.to_owned(),
        actual,
    }))
}

fn effective_ack_wait(ack_wait: Duration, backoff: &[Duration]) -> Duration {
    backoff.first().copied().unwrap_or(ack_wait)
}

async fn get_or_create_validated_stream(
    context: &Context,
    config: StreamConfig,
) -> error::Result<JetStream> {
    let expected = config.num_replicas;
    let stream = context.get_or_create_stream(config).await?;
    let actual = stream.cached_info().config.num_replicas;

    if actual != expected {
        return Err(error::Error::Internal(Box::new(
            NatsStreamReplicaMismatch {
                stream: stream.cached_info().config.name.clone(),
                expected,
                actual,
            },
        )));
    }

    Ok(stream)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_nats::jetstream::consumer::pull::Config as ConsumerConfig;
    use async_nats::jetstream::consumer::{AckPolicy, DeliverPolicy};

    use super::{
        effective_ack_wait, validate_view_consumer, view_consumer_config, NatsStoreOptions,
        NatsStreamReplicas, DEFAULT_ACK_WAIT, VIEW_PROJECTOR_ID_METADATA_KEY,
        VIEW_PROJECTOR_VERSION_METADATA_KEY,
    };
    use crate::event::event_model::ViewProjectorIdentity;

    #[test]
    fn default_options_preserve_one_replica() {
        assert_eq!(
            NatsStoreOptions::default().stream_replicas(),
            NatsStreamReplicas::One
        );
        assert_eq!(NatsStreamReplicas::One.count(), 1);
    }

    #[test]
    fn replicated_options_request_three_replicas() {
        assert_eq!(
            NatsStoreOptions::replicated().stream_replicas(),
            NatsStreamReplicas::Three
        );
        assert_eq!(NatsStreamReplicas::Three.count(), 3);
    }

    #[test]
    fn view_consumers_are_single_flight_and_retry_without_skipping() {
        let ack_wait = Duration::from_secs(7);
        let config = view_consumer_config(
            ConsumerConfig {
                ack_policy: AckPolicy::None,
                ack_wait,
                max_ack_pending: 99,
                max_deliver: 3,
                deliver_policy: DeliverPolicy::New,
                ..Default::default()
            },
            "view-name".to_owned(),
            vec!["events.a.*".to_owned(), "events.b.*".to_owned()],
            &ViewProjectorIdentity::new("orders-summary", 1),
        );

        assert_eq!(config.durable_name.as_deref(), Some("view-name"));
        assert_eq!(config.filter_subjects, ["events.a.*", "events.b.*"]);
        assert_eq!(config.deliver_policy, DeliverPolicy::New);
        assert_eq!(config.ack_policy, AckPolicy::Explicit);
        assert_eq!(config.ack_wait, ack_wait);
        assert_eq!(config.max_ack_pending, 1);
        assert_eq!(config.max_deliver, -1);
        assert_eq!(
            config.metadata.get(VIEW_PROJECTOR_ID_METADATA_KEY),
            Some(&"orders-summary".to_owned())
        );
        assert_eq!(
            config.metadata.get(VIEW_PROJECTOR_VERSION_METADATA_KEY),
            Some(&"1".to_owned())
        );
    }

    #[test]
    fn view_consumers_make_the_server_ack_wait_default_explicit() {
        let config = view_consumer_config(
            ConsumerConfig::default(),
            "view-name".to_owned(),
            vec!["events.a.*".to_owned()],
            &ViewProjectorIdentity::new("orders-summary", 1),
        );

        assert_eq!(config.ack_wait, DEFAULT_ACK_WAIT);
    }

    #[test]
    fn view_progress_uses_the_first_backoff_as_the_effective_ack_wait() {
        let first_backoff = Duration::from_millis(250);
        let config = ConsumerConfig {
            ack_wait: Duration::from_secs(30),
            backoff: vec![first_backoff, Duration::from_secs(1)],
            ..Default::default()
        };

        assert_eq!(
            effective_ack_wait(config.ack_wait, &config.backoff),
            first_backoff
        );
    }

    #[test]
    fn view_identity_validation_rejects_id_version_metadata_and_filter_conflicts() {
        let identity = ViewProjectorIdentity::new("orders-summary", 2);
        let expected = view_consumer_config(
            ConsumerConfig::default(),
            "view-name".to_owned(),
            vec!["events.a.*".to_owned()],
            &identity,
        );
        let mut id_conflict = async_nats::jetstream::consumer::Config {
            filter_subjects: expected.filter_subjects.clone(),
            metadata: expected.metadata.clone(),
            ..Default::default()
        };
        id_conflict.metadata.insert(
            VIEW_PROJECTOR_ID_METADATA_KEY.to_owned(),
            "other-summary".to_owned(),
        );
        assert!(validate_view_consumer("view-name", &id_conflict, &expected, &identity).is_err());

        let mut version_conflict = async_nats::jetstream::consumer::Config {
            filter_subjects: expected.filter_subjects.clone(),
            metadata: expected.metadata.clone(),
            ..Default::default()
        };
        version_conflict.metadata.insert(
            VIEW_PROJECTOR_VERSION_METADATA_KEY.to_owned(),
            "3".to_owned(),
        );
        assert!(
            validate_view_consumer("view-name", &version_conflict, &expected, &identity).is_err()
        );

        let missing_metadata = async_nats::jetstream::consumer::Config {
            filter_subjects: expected.filter_subjects.clone(),
            ..Default::default()
        };
        assert!(
            validate_view_consumer("view-name", &missing_metadata, &expected, &identity).is_err()
        );

        let different_filters = async_nats::jetstream::consumer::Config {
            filter_subjects: vec!["events.b.*".to_owned()],
            metadata: expected.metadata.clone(),
            ..Default::default()
        };
        assert!(
            validate_view_consumer("view-name", &different_filters, &expected, &identity).is_err()
        );

        let matching = async_nats::jetstream::consumer::Config {
            filter_subjects: expected.filter_subjects.clone(),
            metadata: expected.metadata.clone(),
            ..Default::default()
        };
        assert!(validate_view_consumer("view-name", &matching, &expected, &identity).is_ok());
    }
}
