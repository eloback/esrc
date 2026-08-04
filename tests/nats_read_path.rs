#![cfg(feature = "nats")]

use std::collections::HashMap;
use std::env;
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, Result};
use esrc::event::{Publish, ReplayOne, Sequence};
use esrc::nats::NatsStore;
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Envelope, Error, Event};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const OPERATION_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize, DeserializeVersion, Event, PartialEq, Serialize, SerializeVersion)]
enum CounterEvent {
    Added(u64),
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn replay_one_is_ordered_exclusive_and_finite_across_sequence_gaps() -> Result<()> {
    let url = env::var("NATS_URL").context("NATS_URL is required")?;
    let user = env::var("NATS_USER").context("NATS_USER is required")?;
    let password = env::var("NATS_PASSWORD").context("NATS_PASSWORD is required")?;
    let prefix: &'static str =
        Box::leak(format!("MILESTONE0003{}", Uuid::now_v7().simple()).into_boxed_str());

    let client = async_nats::ConnectOptions::with_user_and_password(user, password)
        .connect(url)
        .await?;
    let context = async_nats::jetstream::new(client);
    let mut store = NatsStore::try_new(context.clone(), prefix).await?;

    let scenario_result = tokio::time::timeout(OPERATION_TIMEOUT, run_scenario(&mut store))
        .await
        .context("read-path scenario timed out")?;
    let cleanup_result = context.delete_stream(prefix).await;

    cleanup_result.context("failed to delete the milestone's synthetic stream")?;
    let measurements = scenario_result?;
    println!(
        "initial_sequences={:?} after_first_sequences={:?} captured_upper_bound={} post_bound_sequence={} order=PASS exclusive=PASS finite=PASS cleanup=PASS",
        measurements.initial_sequences,
        measurements.after_first_sequences,
        measurements.captured_upper_bound,
        measurements.post_bound_sequence,
    );

    Ok(())
}

struct Measurements {
    initial_sequences: Vec<u64>,
    after_first_sequences: Vec<u64>,
    captured_upper_bound: u64,
    post_bound_sequence: u64,
}

async fn run_scenario(store: &mut NatsStore) -> Result<Measurements> {
    let target_id = Uuid::now_v7();
    let other_id = Uuid::now_v7();
    let mut metadata = HashMap::new();
    metadata.insert("fixture".to_owned(), "milestone-0003".to_owned());
    let first = store
        .publish(
            target_id,
            Sequence::new(),
            CounterEvent::Added(1),
            Some(metadata),
        )
        .await?;
    let other_first = store
        .publish(other_id, Sequence::new(), CounterEvent::Added(101), None)
        .await?;
    let second = store
        .publish(target_id, first, CounterEvent::Added(2), None)
        .await?;
    let _ = store
        .publish(other_id, other_first, CounterEvent::Added(102), None)
        .await?;
    let third = store
        .publish(target_id, second, CounterEvent::Added(3), None)
        .await?;

    let field_replay = store
        .replay_one::<CounterEvent>(target_id, Sequence::new())
        .await?;
    futures::pin_mut!(field_replay);
    let field_envelope = field_replay
        .next()
        .await
        .context("expected the first stored event")??;
    anyhow::ensure!(field_envelope.id() == target_id, "raw envelope ID changed");
    anyhow::ensure!(
        field_envelope.sequence() == first,
        "raw envelope sequence changed"
    );
    anyhow::ensure!(
        field_envelope.name() == CounterEvent::name(),
        "raw envelope event name changed"
    );
    anyhow::ensure!(
        field_envelope.get_metadata("fixture") == Some("milestone-0003"),
        "raw envelope metadata changed"
    );
    anyhow::ensure!(
        SystemTime::now()
            .duration_since(field_envelope.timestamp())
            .is_ok_and(|age| age <= Duration::from_secs(60)),
        "raw envelope timestamp was not the recent stored publish time"
    );
    anyhow::ensure!(
        field_envelope.deserialize::<CounterEvent>()? == CounterEvent::Added(1),
        "raw envelope versioned deserialization changed"
    );
    anyhow::ensure!(
        matches!(field_envelope.ack().await, Err(Error::Invalid)),
        "a consumer-free stored message was incorrectly acknowledgeable"
    );

    let mut concurrent_writer = store.clone();
    let initial_replay = store
        .replay_one::<CounterEvent>(target_id, Sequence::new())
        .await?;
    let post_bound = concurrent_writer
        .publish(target_id, third, CounterEvent::Added(4), None)
        .await?;

    let (initial_sequences, initial_events) = collect(initial_replay).await?;
    anyhow::ensure!(
        initial_sequences == [u64::from(first), u64::from(second), u64::from(third)],
        "initial replay did not preserve target event sequence across gaps"
    );
    anyhow::ensure!(
        initial_events
            == [
                CounterEvent::Added(1),
                CounterEvent::Added(2),
                CounterEvent::Added(3),
            ],
        "initial replay returned a missing, duplicate, reordered, or cross-aggregate event"
    );

    let after_first = store.replay_one::<CounterEvent>(target_id, first).await?;
    let (after_first_sequences, after_first_events) = collect(after_first).await?;
    anyhow::ensure!(
        after_first_sequences == [u64::from(second), u64::from(third), u64::from(post_bound),],
        "exclusive replay returned the wrong stream sequences"
    );
    anyhow::ensure!(
        after_first_events
            == [
                CounterEvent::Added(2),
                CounterEvent::Added(3),
                CounterEvent::Added(4),
            ],
        "exclusive replay returned the wrong events"
    );

    let empty_after_latest = store
        .replay_one::<CounterEvent>(target_id, post_bound)
        .await?;
    let (empty_sequences, empty_events) = collect(empty_after_latest).await?;
    anyhow::ensure!(
        empty_sequences.is_empty() && empty_events.is_empty(),
        "replay after the latest sequence was not empty"
    );

    let missing_aggregate = store
        .replay_one::<CounterEvent>(Uuid::now_v7(), Sequence::new())
        .await?;
    let (missing_sequences, missing_events) = collect(missing_aggregate).await?;
    anyhow::ensure!(
        missing_sequences.is_empty() && missing_events.is_empty(),
        "replay for an unknown aggregate was not empty"
    );

    Ok(Measurements {
        initial_sequences,
        after_first_sequences,
        captured_upper_bound: u64::from(third),
        post_bound_sequence: u64::from(post_bound),
    })
}

async fn collect(
    stream: impl futures::Stream<Item = esrc::error::Result<esrc::nats::NatsEnvelope>>,
) -> Result<(Vec<u64>, Vec<CounterEvent>)> {
    futures::pin_mut!(stream);
    let mut sequences = Vec::new();
    let mut events = Vec::new();
    while let Some(envelope) = stream.next().await {
        let envelope = envelope?;
        sequences.push(u64::from(envelope.sequence()));
        events.push(envelope.deserialize()?);
    }
    Ok((sequences, events))
}
