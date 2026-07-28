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
const READ_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize, DeserializeVersion, Event, Serialize, SerializeVersion)]
enum CounterEvent {
    Added(u64),
}

#[derive(Default)]
struct Counter {
    value: u64,
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
            CounterEvent::Added(value) => self.value += value,
        }
        self
    }
}

struct Measurements {
    messages: u64,
    replicas: usize,
    consumer_delta: i64,
    read_latency: Vec<Duration>,
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

    let p50 = percentile(&measurements.read_latency, 50);
    let p95 = percentile(&measurements.read_latency, 95);
    let p99 = percentile(&measurements.read_latency, 99);
    println!(
        "reads={CONCURRENT_READS} messages={} replicas={} consumer_delta={} p50_us={} p95_us={} p99_us={} cleanup=PASS",
        measurements.messages,
        measurements.replicas,
        measurements.consumer_delta,
        p50.as_micros(),
        p95.as_micros(),
        p99.as_micros(),
    );

    Ok(())
}

async fn run_scenario(
    context: &async_nats::jetstream::Context,
    store: &mut NatsStore,
    prefix: &str,
) -> Result<Measurements> {
    let aggregate_id = Uuid::now_v7();
    let mut sequence = Sequence::new();
    for value in [1_u64, 2, 3] {
        sequence = store
            .publish(aggregate_id, sequence, CounterEvent::Added(value), None)
            .await?;
    }

    let mut stream = context.get_stream(prefix).await?;
    let before = stream.info().await?.clone();
    anyhow::ensure!(before.state.messages == 3, "expected three stored events");

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
                aggregate.value == 6,
                "replay produced the wrong aggregate value"
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
    Ok(Measurements {
        messages: after.state.messages,
        replicas: after.config.num_replicas,
        consumer_delta: after_consumers - before_consumers,
        read_latency,
    })
}

fn percentile(sorted_samples: &[Duration], percentile: usize) -> Duration {
    let index = (sorted_samples.len() * percentile).div_ceil(100) - 1;
    sorted_samples[index]
}
