#![cfg(feature = "nats")]

use std::env;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use esrc::aggregate::Root;
use esrc::event::{Publish, ReplayOneExt, Sequence};
use esrc::nats::NatsStore;
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Event};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const CONCURRENT_READS: usize = 100;
const PUBLISHES: usize = 100;
const READ_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize, DeserializeVersion, Event, Serialize, SerializeVersion)]
enum CounterEvent {
    Added(u64),
}

#[derive(Default)]
struct Counter {
    value: u64,
    applied: Vec<u64>,
}

enum CounterCommand {}

#[derive(Debug, thiserror::Error)]
#[error("counter command failed")]
struct CounterError;

impl Aggregate for Counter {
    type Command = CounterCommand;
    type Event = CounterEvent;
    type Error = CounterError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {}
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            CounterEvent::Added(value) => {
                self.value += value;
                self.applied.push(*value);
            },
        }
        self
    }
}

struct Measurements {
    messages: u64,
    bytes: u64,
    replicas: usize,
    leader: String,
    current_followers: usize,
    active_consumers: usize,
    consumer_delta: i64,
    publish_latency: Vec<Duration>,
    read_latency: Vec<Duration>,
    duration: Duration,
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn concurrent_replay_baseline_preserves_events_and_cleans_up() -> Result<()> {
    let url = env::var("NATS_URL").context("NATS_URL is required")?;
    let user = env::var("NATS_USER").context("NATS_USER is required")?;
    let password = env::var("NATS_PASSWORD").context("NATS_PASSWORD is required")?;
    let prefix: &'static str =
        Box::leak(format!("MILESTONE0001{}", Uuid::now_v7().simple()).into_boxed_str());

    let client = async_nats::ConnectOptions::with_user_and_password(user, password)
        .connect(url)
        .await?;
    let context = async_nats::jetstream::new(client);
    let mut store = NatsStore::try_new(context.clone(), prefix).await?;

    let scenario_result = run_scenario(&context, &mut store, prefix).await;
    let cleanup_result = context.delete_stream(prefix).await;

    cleanup_result.context("failed to delete the milestone's synthetic stream")?;
    let measurements = scenario_result?;

    let publish_p50 = percentile(&measurements.publish_latency, 50);
    let publish_p95 = percentile(&measurements.publish_latency, 95);
    let publish_p99 = percentile(&measurements.publish_latency, 99);
    let replay_p50 = percentile(&measurements.read_latency, 50);
    let replay_p95 = percentile(&measurements.read_latency, 95);
    let replay_p99 = percentile(&measurements.read_latency, 99);
    let consumer_creation_rate =
        measurements.consumer_delta as f64 / measurements.duration.as_secs_f64();
    println!(
        "publishes={PUBLISHES} reads={CONCURRENT_READS} messages={} bytes={} replicas={} leader={} current_followers={} active_consumers={} consumer_delta={} consumer_creation_rate_per_s={consumer_creation_rate:.2} errors=0 timeouts=0 publish_p50_us={} publish_p95_us={} publish_p99_us={} replay_p50_us={} replay_p95_us={} replay_p99_us={} duration_ms={} cleanup=PASS",
        measurements.messages,
        measurements.bytes,
        measurements.replicas,
        measurements.leader,
        measurements.current_followers,
        measurements.active_consumers,
        measurements.consumer_delta,
        publish_p50.as_micros(),
        publish_p95.as_micros(),
        publish_p99.as_micros(),
        replay_p50.as_micros(),
        replay_p95.as_micros(),
        replay_p99.as_micros(),
        measurements.duration.as_millis(),
    );
    anyhow::ensure!(
        measurements.consumer_delta == 0,
        "aggregate replay created {} consumers; expected zero",
        measurements.consumer_delta
    );

    Ok(())
}

async fn run_scenario(
    context: &async_nats::jetstream::Context,
    store: &mut NatsStore,
    prefix: &str,
) -> Result<Measurements> {
    let scenario_started = Instant::now();
    let aggregate_id = Uuid::now_v7();
    let mut sequence = Sequence::new();
    let mut publish_latency = Vec::with_capacity(PUBLISHES);
    for value in 1..=PUBLISHES as u64 {
        let publish_started = Instant::now();
        sequence = tokio::time::timeout(
            READ_TIMEOUT,
            store.publish(aggregate_id, sequence, CounterEvent::Added(value), None),
        )
        .await
        .context("publish acknowledgement timed out")??;
        publish_latency.push(publish_started.elapsed());
    }
    publish_latency.sort_unstable();

    let mut stream = context.get_stream(prefix).await?;
    let before = stream.info().await?.clone();
    anyhow::ensure!(
        before.state.messages == PUBLISHES as u64,
        "expected {PUBLISHES} stored events"
    );

    let reads = (0..CONCURRENT_READS).map(|_| {
        let reader = store.clone();
        async move {
            let started = Instant::now();
            let aggregate: Root<Counter> =
                tokio::time::timeout(READ_TIMEOUT, reader.read(aggregate_id))
                    .await
                    .context("aggregate replay timed out")??;
            let elapsed = started.elapsed();

            anyhow::ensure!(
                aggregate.value == (1..=PUBLISHES as u64).sum::<u64>(),
                "replay produced the wrong aggregate value"
            );
            anyhow::ensure!(
                aggregate.applied.len() == PUBLISHES
                    && aggregate.applied.iter().copied().eq(1..=PUBLISHES as u64),
                "replay produced the wrong aggregate event order"
            );
            anyhow::ensure!(
                u64::from(Root::last_sequence(&aggregate)) == u64::from(sequence),
                "replay produced the wrong last sequence"
            );
            Result::<Duration>::Ok(elapsed)
        }
    });

    let mut read_latency = Vec::with_capacity(CONCURRENT_READS);
    for result in join_all(reads).await {
        read_latency.push(result?);
    }
    read_latency.sort_unstable();

    let after = stream.info().await?.clone();
    let after_consumers = i64::try_from(after.state.consumer_count)
        .context("consumer count does not fit in an i64")?;
    let before_consumers = i64::try_from(before.state.consumer_count)
        .context("consumer count does not fit in an i64")?;
    let (leader, current_followers) = after.cluster.as_ref().map_or_else(
        || ("NONE".to_owned(), 0),
        |cluster| {
            (
                cluster.leader.clone().unwrap_or_else(|| "NONE".to_owned()),
                cluster
                    .replicas
                    .iter()
                    .filter(|peer| peer.current && !peer.offline)
                    .count(),
            )
        },
    );
    Ok(Measurements {
        messages: after.state.messages,
        bytes: after.state.bytes,
        replicas: after.config.num_replicas,
        leader,
        current_followers,
        active_consumers: after.state.consumer_count,
        consumer_delta: after_consumers - before_consumers,
        publish_latency,
        read_latency,
        duration: scenario_started.elapsed(),
    })
}

fn percentile(sorted_samples: &[Duration], percentile: usize) -> Duration {
    let index = (sorted_samples.len() * percentile).div_ceil(100) - 1;
    sorted_samples[index]
}
