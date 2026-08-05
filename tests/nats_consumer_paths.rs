#![cfg(feature = "nats")]

use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use async_nats::jetstream::consumer::pull::Config as ConsumerConfig;
use async_nats::jetstream::consumer::AckPolicy;
use async_nats::jetstream::stream::{DiscardPolicy, RetentionPolicy, StorageType};
use async_nats::jetstream::AckKind;
use esrc::event::event_model::{Automation, Translation};
use esrc::nats::{
    DeadLetterMessage, DeadLetterReason, DeadLetterStore, NatsStore, NatsStoreOptions,
};
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Envelope, Event};
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

const OPERATION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize, DeserializeVersion, Event, PartialEq, Serialize, SerializeVersion)]
enum ConsumerPathEvent {
    Added(u64),
}

#[derive(Clone)]
struct ChannelDeadLetterStore {
    sender: mpsc::UnboundedSender<DeadLetterMessage>,
    failures_remaining: Arc<AtomicUsize>,
    attempts: Arc<AtomicUsize>,
}

impl DeadLetterStore for ChannelDeadLetterStore {
    type Error = std::io::Error;

    async fn store_dead_letter(&self, message: DeadLetterMessage) -> Result<(), Self::Error> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        if self
            .failures_remaining
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok()
        {
            return Err(std::io::Error::other(
                "synthetic dead-letter storage failure",
            ));
        }
        let _ = self.sender.send(message);
        Ok(())
    }

    async fn get_dead_letters(
        &self,
        _stream: Option<&str>,
        _consumer: Option<&str>,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<Vec<DeadLetterMessage>, Self::Error> {
        Ok(Vec::new())
    }

    async fn remove_dead_letter(&self, _identifier: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn count_dead_letters(
        &self,
        _stream: Option<&str>,
        _consumer: Option<&str>,
    ) -> Result<u64, Self::Error> {
        Ok(0)
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn durable_subscription_uses_configured_mirror() -> Result<()> {
    let context = connect().await?;
    let suffix = Uuid::now_v7().simple();
    let prefix: &'static str = Box::leak(format!("MILESTONE0010MIRROR{suffix}").into_boxed_str());
    let mirror_name = format!("{prefix}_READ");
    let durable_name = format!("mirror-{suffix}");
    let store = NatsStore::try_new(context.clone(), prefix)
        .await?
        .enable_mirror(mirror_name.clone())
        .await?;

    let scenario_result =
        run_mirror_scenario(&context, store.clone(), prefix, &mirror_name, &durable_name).await;
    let mirror_cleanup = context.delete_stream(&mirror_name).await;
    let writer_cleanup = context.delete_stream(prefix).await;

    mirror_cleanup.context("failed to delete the synthetic mirror")?;
    writer_cleanup.context("failed to delete the synthetic writer stream")?;
    scenario_result?;
    println!(
        "writer_consumer=ABSENT mirror_consumer=PRESENT deliveries=1 duplicates=0 ack=CONFIRMED cleanup=PASS"
    );
    Ok(())
}

async fn run_mirror_scenario(
    context: &async_nats::jetstream::Context,
    mut store: NatsStore,
    prefix: &str,
    mirror_name: &str,
    durable_name: &str,
) -> Result<()> {
    let subscriber = store.clone();
    let mut deliveries = std::pin::pin!(
        subscriber
            .durable_subscribe::<ConsumerPathEvent>(durable_name)
            .await?
    );

    let writer = context.get_stream(prefix).await?;
    let mirror = context.get_stream(mirror_name).await?;
    anyhow::ensure!(
        writer
            .get_consumer::<ConsumerConfig>(durable_name)
            .await
            .is_err(),
        "durable consumer was attached to the authoritative writer stream"
    );
    mirror
        .get_consumer::<ConsumerConfig>(durable_name)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("durable consumer is missing from the configured mirror")?;

    let aggregate_id = Uuid::now_v7();
    store
        .publish_to_automation(aggregate_id, ConsumerPathEvent::Added(7))
        .await?;
    let envelope = tokio::time::timeout(OPERATION_TIMEOUT, deliveries.next())
        .await
        .context("timed out waiting for the mirrored event")?
        .context("mirror subscription ended")??;
    anyhow::ensure!(envelope.id() == aggregate_id, "mirrored event ID changed");
    anyhow::ensure!(
        envelope.deserialize::<ConsumerPathEvent>()? == ConsumerPathEvent::Added(7),
        "mirrored event payload changed"
    );
    envelope.ack().await?;
    anyhow::ensure!(
        tokio::time::timeout(Duration::from_millis(250), deliveries.next())
            .await
            .is_err(),
        "mirror subscription delivered an unexpected duplicate"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn dead_letter_automation_captures_terminated_advisory() -> Result<()> {
    let context = connect().await?;
    let suffix = Uuid::now_v7().simple();
    let prefix: &'static str = Box::leak(format!("MILESTONE0010DLQ{suffix}").into_boxed_str());
    let source_consumer = format!("source-{suffix}");
    let advisory_consumer = format!("advisory-{suffix}");
    let store =
        NatsStore::try_new_with_options(context.clone(), prefix, NatsStoreOptions::replicated())
            .await?
            .update_durable_consumer_option(ConsumerConfig {
                ack_wait: Duration::from_millis(100),
                ..Default::default()
            });
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let attempts = Arc::new(AtomicUsize::new(0));
    let dead_letter_store = ChannelDeadLetterStore {
        sender,
        failures_remaining: Arc::new(AtomicUsize::new(1)),
        attempts: attempts.clone(),
    };

    let scenario_result = run_dead_letter_scenario(
        &context,
        store.clone(),
        dead_letter_store,
        &mut receiver,
        prefix,
        &source_consumer,
        &advisory_consumer,
    )
    .await;
    store.wait_graceful_shutdown().await;
    cleanup_prefixed_streams(&context, prefix).await?;
    scenario_result?;
    anyhow::ensure!(
        attempts.load(Ordering::SeqCst) == 3,
        "two advisories did not include exactly one redelivery after the injected storage failure"
    );
    println!(
        "advisories=MSG_TERMINATED,MAX_DELIVERIES advisory_subjects=2 retention=workqueue storage=file unbounded=true replicas=3 storage_attempts=3 dead_letters=2 duplicates=0 messages_after_ack=0 ack_after_store=CONFIRMED cleanup=PASS"
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_dead_letter_scenario(
    context: &async_nats::jetstream::Context,
    mut store: NatsStore,
    dead_letter_store: ChannelDeadLetterStore,
    receiver: &mut mpsc::UnboundedReceiver<DeadLetterMessage>,
    prefix: &str,
    source_consumer: &str,
    advisory_consumer: &str,
) -> Result<()> {
    store
        .run_dead_letter_automation(
            dead_letter_store,
            advisory_consumer,
            prefix,
            source_consumer,
        )
        .await?;

    let advisory_stream_name = format!("{prefix}_DLQ_{source_consumer}");
    let mut advisory_stream = context.get_stream(&advisory_stream_name).await?;
    let advisory_info = advisory_stream.info().await?.clone();
    anyhow::ensure!(
        advisory_info.config.subjects
            == [
                format!("$JS.EVENT.ADVISORY.CONSUMER.MAX_DELIVERIES.{prefix}.{source_consumer}"),
                format!("$JS.EVENT.ADVISORY.CONSUMER.MSG_TERMINATED.{prefix}.{source_consumer}"),
            ],
        "dedicated advisory stream has the wrong subjects"
    );
    anyhow::ensure!(
        advisory_info.config.retention == RetentionPolicy::WorkQueue,
        "dedicated advisory stream is not a work queue"
    );
    anyhow::ensure!(
        advisory_info.config.discard == DiscardPolicy::New,
        "dedicated advisory stream can discard an older pending advisory"
    );
    anyhow::ensure!(
        advisory_info.config.storage == StorageType::File,
        "dedicated advisory stream is not persisted to file storage"
    );
    anyhow::ensure!(
        advisory_info.config.max_messages <= 0
            && advisory_info.config.max_bytes <= 0
            && advisory_info.config.max_messages_per_subject <= 0
            && advisory_info.config.max_age.is_zero(),
        "dedicated advisory stream has an unapproved retention limit"
    );
    anyhow::ensure!(
        advisory_info.config.num_replicas == 3,
        "dedicated advisory stream does not match the store replica policy"
    );
    advisory_stream
        .get_consumer::<ConsumerConfig>(advisory_consumer)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("dedicated advisory consumer is missing")?;

    let source_stream = context.get_stream(prefix).await?;
    let source = source_stream
        .create_consumer(ConsumerConfig {
            durable_name: Some(source_consumer.to_owned()),
            ack_policy: AckPolicy::Explicit,
            max_deliver: 1,
            filter_subject: format!("{prefix}.{}.*", ConsumerPathEvent::name()),
            ..Default::default()
        })
        .await?;
    let mut source_messages = source.messages().await?;
    store
        .publish_to_automation(Uuid::now_v7(), ConsumerPathEvent::Added(11))
        .await?;
    let source_message = tokio::time::timeout(OPERATION_TIMEOUT, source_messages.next())
        .await
        .context("timed out waiting for the source event")?
        .context("source consumer ended")??;
    source_message
        .ack_with(AckKind::Term)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    let dead_letter = tokio::time::timeout(OPERATION_TIMEOUT, receiver.recv())
        .await
        .context("timed out waiting for the dead-letter record")?
        .context("dead-letter channel ended")?;
    anyhow::ensure!(dead_letter.stream == prefix, "dead-letter stream changed");
    anyhow::ensure!(
        dead_letter.consumer == source_consumer,
        "dead-letter consumer changed"
    );
    anyhow::ensure!(
        matches!(dead_letter.reason, DeadLetterReason::MessageTerminated),
        "dead-letter reason was not message termination"
    );

    store
        .publish_to_automation(Uuid::now_v7(), ConsumerPathEvent::Added(13))
        .await?;
    let max_delivery_message = tokio::time::timeout(OPERATION_TIMEOUT, source_messages.next())
        .await
        .context("timed out waiting for the max-delivery source event")?
        .context("source consumer ended before max-delivery event")??;
    max_delivery_message
        .ack_with(AckKind::Nak(None))
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let max_delivery_dead_letter = tokio::time::timeout(OPERATION_TIMEOUT, receiver.recv())
        .await
        .context("timed out waiting for the max-delivery dead-letter record")?
        .context("dead-letter channel ended before max-delivery record")?;
    anyhow::ensure!(
        max_delivery_dead_letter.stream == prefix,
        "max-delivery dead-letter stream changed"
    );
    anyhow::ensure!(
        max_delivery_dead_letter.consumer == source_consumer,
        "max-delivery dead-letter consumer changed"
    );
    anyhow::ensure!(
        matches!(
            max_delivery_dead_letter.reason,
            DeadLetterReason::MaxDeliveriesReached
        ),
        "dead-letter reason was not maximum deliveries"
    );
    anyhow::ensure!(
        tokio::time::timeout(Duration::from_millis(250), receiver.recv())
            .await
            .is_err(),
        "terminated advisory produced an unexpected duplicate dead-letter record"
    );
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        loop {
            if advisory_stream.info().await?.state.messages == 0 {
                return Result::<()>::Ok(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .context("acknowledged advisory remained in the work queue")??;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn dead_letter_advisory_survives_automation_restart() -> Result<()> {
    let context = connect().await?;
    let suffix = Uuid::now_v7().simple();
    let prefix: &'static str = Box::leak(format!("MILESTONE0012RESTART{suffix}").into_boxed_str());
    let source_consumer = format!("source-{suffix}");
    let advisory_consumer = format!("advisory-{suffix}");
    let first_store = NatsStore::try_new(context.clone(), prefix)
        .await?
        .update_durable_consumer_option(ConsumerConfig {
            ack_wait: Duration::from_millis(500),
            ..Default::default()
        });
    let (first_sender, _first_receiver) = mpsc::unbounded_channel();
    let first_attempts = Arc::new(AtomicUsize::new(0));
    let failing_store = ChannelDeadLetterStore {
        sender: first_sender,
        failures_remaining: Arc::new(AtomicUsize::new(1)),
        attempts: first_attempts.clone(),
    };

    let seed_result = seed_pending_advisory(
        &context,
        first_store.clone(),
        failing_store,
        first_attempts,
        prefix,
        &source_consumer,
        &advisory_consumer,
    )
    .await;
    first_store.wait_graceful_shutdown().await;
    if let Err(error) = seed_result {
        cleanup_prefixed_streams(&context, prefix).await?;
        return Err(error);
    }

    let second_store = NatsStore::try_new(context.clone(), prefix)
        .await?
        .update_durable_consumer_option(ConsumerConfig {
            ack_wait: Duration::from_millis(100),
            ..Default::default()
        });
    let (second_sender, mut second_receiver) = mpsc::unbounded_channel();
    let second_attempts = Arc::new(AtomicUsize::new(0));
    let recovering_store = ChannelDeadLetterStore {
        sender: second_sender,
        failures_remaining: Arc::new(AtomicUsize::new(0)),
        attempts: second_attempts.clone(),
    };
    let recovery_result = recover_pending_advisory(
        &context,
        second_store.clone(),
        recovering_store,
        &mut second_receiver,
        prefix,
        &source_consumer,
        &advisory_consumer,
    )
    .await;
    second_store.wait_graceful_shutdown().await;
    cleanup_prefixed_streams(&context, prefix).await?;
    recovery_result?;
    anyhow::ensure!(
        second_attempts.load(Ordering::SeqCst) == 1,
        "recovered advisory was not stored exactly once"
    );
    println!(
        "automation_restart=PASS pending_before_restart=1 recovered_dead_letters=1 recovery_duplicates=0 messages_after_ack=0 cleanup=PASS"
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn seed_pending_advisory(
    context: &async_nats::jetstream::Context,
    mut store: NatsStore,
    dead_letter_store: ChannelDeadLetterStore,
    attempts: Arc<AtomicUsize>,
    prefix: &str,
    source_consumer: &str,
    advisory_consumer: &str,
) -> Result<()> {
    store
        .run_dead_letter_automation(
            dead_letter_store,
            advisory_consumer,
            prefix,
            source_consumer,
        )
        .await?;
    let source_stream = context.get_stream(prefix).await?;
    let source = source_stream
        .create_consumer(ConsumerConfig {
            durable_name: Some(source_consumer.to_owned()),
            ack_policy: AckPolicy::Explicit,
            filter_subject: format!("{prefix}.{}.*", ConsumerPathEvent::name()),
            ..Default::default()
        })
        .await?;
    let mut source_messages = source.messages().await?;
    store
        .publish_to_automation(Uuid::now_v7(), ConsumerPathEvent::Added(17))
        .await?;
    let source_message = tokio::time::timeout(OPERATION_TIMEOUT, source_messages.next())
        .await
        .context("timed out waiting for restart source event")?
        .context("restart source consumer ended")??;
    source_message
        .ack_with(AckKind::Term)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        while attempts.load(Ordering::SeqCst) < 1 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .context("first dead-letter storage attempt did not occur")?;

    let mut advisory_stream = context
        .get_stream(format!("{prefix}_DLQ_{source_consumer}"))
        .await?;
    anyhow::ensure!(
        advisory_stream.info().await?.state.messages == 1,
        "failed dead-letter storage did not leave one pending advisory"
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn recover_pending_advisory(
    context: &async_nats::jetstream::Context,
    store: NatsStore,
    dead_letter_store: ChannelDeadLetterStore,
    receiver: &mut mpsc::UnboundedReceiver<DeadLetterMessage>,
    prefix: &str,
    source_consumer: &str,
    advisory_consumer: &str,
) -> Result<()> {
    store
        .run_dead_letter_automation(
            dead_letter_store,
            advisory_consumer,
            prefix,
            source_consumer,
        )
        .await?;
    let recovered = tokio::time::timeout(OPERATION_TIMEOUT, receiver.recv())
        .await
        .context("timed out recovering the pending advisory")?
        .context("recovery dead-letter channel ended")?;
    anyhow::ensure!(
        matches!(recovered.reason, DeadLetterReason::MessageTerminated),
        "recovered advisory reason changed"
    );
    anyhow::ensure!(
        tokio::time::timeout(Duration::from_millis(250), receiver.recv())
            .await
            .is_err(),
        "recovered advisory produced a duplicate dead-letter record"
    );

    let mut advisory_stream = context
        .get_stream(format!("{prefix}_DLQ_{source_consumer}"))
        .await?;
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        loop {
            if advisory_stream.info().await?.state.messages == 0 {
                return Result::<()>::Ok(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .context("recovered advisory remained pending after storage")??;
    Ok(())
}

async fn cleanup_prefixed_streams(
    context: &async_nats::jetstream::Context,
    prefix: &str,
) -> Result<()> {
    let stream_names = context.stream_names().try_collect::<Vec<_>>().await?;
    let mut cleanup_result = Ok(());
    for stream_name in stream_names
        .into_iter()
        .filter(|stream_name| stream_name.starts_with(prefix))
    {
        if let Err(error) = context.delete_stream(&stream_name).await {
            cleanup_result = Err(error);
        }
    }
    cleanup_result.context("failed to delete a synthetic DLQ stream")?;
    Ok(())
}

async fn connect() -> Result<async_nats::jetstream::Context> {
    let urls = env::var("NATS_URL")
        .context("NATS_URL is required")?
        .split(',')
        .map(str::trim)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let user = env::var("NATS_USER").context("NATS_USER is required")?;
    let password = env::var("NATS_PASSWORD").context("NATS_PASSWORD is required")?;
    let client = async_nats::ConnectOptions::with_user_and_password(user, password)
        .connect(urls)
        .await?;
    Ok(async_nats::jetstream::new(client))
}
