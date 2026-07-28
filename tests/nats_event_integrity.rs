#![cfg(feature = "nats")]

use std::collections::HashMap;
use std::env;
use std::time::Duration;

use anyhow::{Context as _, Result};
use esrc::aggregate::Root;
use esrc::event::event_model::{Automation, Translation};
use esrc::event::{Publish, PublishExt, ReplayOneExt, Sequence};
use esrc::nats::NatsStore;
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Error, Event};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const OPERATION_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize, DeserializeVersion, Event, Serialize, SerializeVersion)]
enum CounterEvent {
    Added(u64),
}

enum CounterCommand {
    Add(u64),
}

#[derive(Default)]
struct Counter {
    value: u64,
}

#[derive(Debug, thiserror::Error)]
#[error("counter command failed")]
struct CounterError;

impl Aggregate for Counter {
    type Command = CounterCommand;
    type Event = CounterEvent;
    type Error = CounterError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            CounterCommand::Add(value) => Ok(CounterEvent::Added(value)),
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            CounterEvent::Added(value) => self.value += value,
        }
        self
    }
}

struct Measurements {
    write_sequence_1: u64,
    write_sequence_2: u64,
    snapshot_sequence: u64,
    deduplicated_sequence: u64,
    messages_after_retry: u64,
    translation_message_delta: u64,
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn event_integrity_semantics() -> Result<()> {
    let url = env::var("NATS_URL").context("NATS_URL is required")?;
    let user = env::var("NATS_USER").context("NATS_USER is required")?;
    let password = env::var("NATS_PASSWORD").context("NATS_PASSWORD is required")?;
    let prefix: &'static str =
        Box::leak(format!("MILESTONE0002{}", Uuid::now_v7().simple()).into_boxed_str());

    let client = async_nats::ConnectOptions::with_user_and_password(user, password)
        .connect(url)
        .await?;
    let context = async_nats::jetstream::new(client);
    let mut store = NatsStore::try_new(context.clone(), prefix).await?;

    let scenario_result = tokio::time::timeout(
        OPERATION_TIMEOUT,
        run_scenario(&context, &mut store, prefix),
    )
    .await
    .context("event-integrity scenario timed out")?;
    let cleanup_result = context.delete_stream(prefix).await;

    cleanup_result.context("failed to delete the milestone's synthetic stream")?;
    let measurements = scenario_result?;
    println!(
        "write_seq_1={} write_seq_2={} snapshot_seq={} dedup_seq={} messages_after_retry={} translation_delta={} ack=CONFIRMED cleanup=PASS",
        measurements.write_sequence_1,
        measurements.write_sequence_2,
        measurements.snapshot_sequence,
        measurements.deduplicated_sequence,
        measurements.messages_after_retry,
        measurements.translation_message_delta,
    );

    Ok(())
}

async fn run_scenario(
    context: &async_nats::jetstream::Context,
    store: &mut NatsStore,
    prefix: &str,
) -> Result<Measurements> {
    let write_id = Uuid::now_v7();
    let root = Root::<Counter>::new(write_id);
    let root = store
        .write(root, CounterEvent::Added(1), None)
        .await
        .context("first PublishExt write failed")?;
    let write_sequence_1 = u64::from(Root::last_sequence(&root));
    anyhow::ensure!(
        write_sequence_1 > 0,
        "first write returned a stale sequence"
    );

    let snapshot = store
        .try_write(root, CounterCommand::Add(2), None)
        .await
        .context("second PublishExt write failed")?;
    let write_sequence_2 = u64::from(Root::last_sequence(&snapshot));
    anyhow::ensure!(
        write_sequence_2 > write_sequence_1,
        "second write did not advance the acknowledged sequence"
    );
    anyhow::ensure!(snapshot.value == 3, "write chain produced the wrong state");

    let snapshot_sequence = store
        .publish(
            write_id,
            Root::last_sequence(&snapshot),
            CounterEvent::Added(3),
            None,
        )
        .await
        .context("post-snapshot publish failed")?;
    let refreshed = store
        .read_after(snapshot)
        .await
        .context("snapshot refresh failed")?;
    anyhow::ensure!(
        refreshed.value == 6,
        "snapshot refresh produced the wrong state"
    );
    anyhow::ensure!(
        Root::last_sequence(&refreshed) == snapshot_sequence,
        "snapshot refresh returned the wrong sequence"
    );

    let retry_id = Uuid::now_v7();
    let mut metadata_first = HashMap::new();
    metadata_first.insert("zeta".to_owned(), "last".to_owned());
    metadata_first.insert("alpha".to_owned(), "first".to_owned());
    let deduplicated_sequence = store
        .publish(
            retry_id,
            Sequence::new(),
            CounterEvent::Added(7),
            Some(metadata_first),
        )
        .await
        .context("initial deduplication publish failed")?;

    let mut metadata_retry = HashMap::new();
    metadata_retry.insert("alpha".to_owned(), "first".to_owned());
    metadata_retry.insert("zeta".to_owned(), "last".to_owned());
    let retry_sequence = store
        .publish(
            retry_id,
            Sequence::new(),
            CounterEvent::Added(7),
            Some(metadata_retry),
        )
        .await
        .context("exact OCC retry failed")?;
    anyhow::ensure!(
        retry_sequence == deduplicated_sequence,
        "exact retry did not return the original sequence"
    );

    let conflict = store
        .publish(retry_id, Sequence::new(), CounterEvent::Added(8), None)
        .await;
    anyhow::ensure!(
        matches!(conflict, Err(Error::Conflict)),
        "different event at a stale sequence was not rejected"
    );

    let mut stream = context.get_stream(prefix).await?;
    let after_retry = stream.info().await?.clone();
    anyhow::ensure!(
        after_retry.state.messages == 4,
        "exact retry changed the expected stream message count"
    );

    let subscriber = store.clone();
    let consumer_name = format!("integrity-{}", Uuid::now_v7().simple());
    let mut deliveries = std::pin::pin!(
        subscriber
            .durable_subscribe::<CounterEvent>(&consumer_name)
            .await?
    );
    let before_translation = stream.info().await?.state.messages;
    store
        .publish_to_automation(Uuid::now_v7(), CounterEvent::Added(11))
        .await?;
    let envelope = tokio::time::timeout(Duration::from_secs(5), deliveries.next())
        .await
        .context("timed out waiting for the translated event")?
        .context("translation subscription ended")??;
    envelope.ack().await?;

    let after_translation = stream.info().await?.state.messages;
    let translation_message_delta = after_translation - before_translation;
    anyhow::ensure!(
        translation_message_delta == 1,
        "translation returned without exactly one stored event"
    );

    Ok(Measurements {
        write_sequence_1,
        write_sequence_2,
        snapshot_sequence: u64::from(snapshot_sequence),
        deduplicated_sequence: u64::from(deduplicated_sequence),
        messages_after_retry: after_retry.state.messages,
        translation_message_delta,
    })
}
