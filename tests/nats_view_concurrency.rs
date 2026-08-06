#![cfg(feature = "nats")]

use std::env;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use async_nats::jetstream::consumer::pull::Config as ConsumerConfig;
use async_nats::jetstream::consumer::{AckPolicy, DeliverPolicy};
use esrc::event::event_model::{Translation, ViewAutomation};
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Envelope, Event};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use uuid::Uuid;

const BACKLOG_EVENTS: usize = 400;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Deserialize, DeserializeVersion, Event, Serialize, SerializeVersion)]
enum ViewEvent {
    Applied(u64),
}

#[derive(Debug, thiserror::Error)]
#[error("synthetic projection failure")]
struct ProjectionError;

#[derive(Default)]
struct ProbeState {
    applied: Mutex<Vec<(u64, u64)>>,
    attempts: AtomicUsize,
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
    failed_once: AtomicBool,
    latencies: Mutex<Vec<Duration>>,
    changed: Notify,
}

impl ProbeState {
    fn enter(&self) {
        let current = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_in_flight.fetch_max(current, Ordering::SeqCst);
        self.attempts.fetch_add(1, Ordering::SeqCst);
    }

    fn leave(&self) {
        self.current_in_flight.fetch_sub(1, Ordering::SeqCst);
        self.changed.notify_waiters();
    }

    fn record(&self, value: u64, sequence: u64, elapsed: Duration) {
        self.applied
            .lock()
            .expect("applied lock should not be poisoned")
            .push((value, sequence));
        self.latencies
            .lock()
            .expect("latency lock should not be poisoned")
            .push(elapsed);
    }

    fn applied(&self) -> Vec<(u64, u64)> {
        self.applied
            .lock()
            .expect("applied lock should not be poisoned")
            .clone()
    }
}

#[derive(Clone)]
struct ProbeProjector {
    state: Arc<ProbeState>,
    delay: Duration,
    fail_once_on: Option<u64>,
}

#[derive(Clone, Default)]
struct LocalStateProjector {
    local_count: usize,
    observed: Arc<Mutex<Vec<usize>>>,
    changed: Arc<Notify>,
}

impl Project for LocalStateProjector {
    type EventGroup = ViewEvent;
    type Error = ProjectionError;

    async fn project<'de, E>(
        &mut self,
        _context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: Envelope + Sync,
    {
        self.local_count += 1;
        self.observed
            .lock()
            .expect("observed lock should not be poisoned")
            .push(self.local_count);
        self.changed.notify_waiters();
        Ok(())
    }
}

#[derive(Clone)]
struct ConflictingProjector {
    state: Arc<ProbeState>,
}

impl Project for ConflictingProjector {
    type EventGroup = ViewEvent;
    type Error = ProjectionError;

    async fn project<'de, E>(
        &mut self,
        _context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: Envelope + Sync,
    {
        self.state.attempts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

impl Project for ProbeProjector {
    type EventGroup = ViewEvent;
    type Error = ProjectionError;

    async fn project<'de, E>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: Envelope + Sync,
    {
        let ViewEvent::Applied(value) = *context;
        let sequence = u64::from(Context::sequence(&context));
        let started = Instant::now();
        self.state.enter();
        tokio::time::sleep(self.delay).await;

        if self.fail_once_on == Some(value) && !self.state.failed_once.swap(true, Ordering::SeqCst)
        {
            self.state.leave();
            return Err(ProjectionError);
        }

        self.state.record(value, sequence, started.elapsed());
        self.state.leave();
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn live_views_are_cluster_wide_sequential() -> Result<()> {
    let context_one = connect().await?;
    let context_two = connect().await?;
    let prefix = leaked_prefix("MILESTONE0014SEQUENCE");
    let durable = format!("view-sequence-{}", Uuid::now_v7().simple());
    let store_one = NatsStore::try_new(context_one.clone(), prefix).await?;
    let store_two = NatsStore::try_new(context_two, prefix).await?;
    let state = Arc::new(ProbeState::default());
    let projector = ProbeProjector {
        state: state.clone(),
        delay: Duration::from_millis(25),
        fail_once_on: None,
    };

    let first = start_view(store_one.clone(), projector.clone(), durable.clone());
    wait_for_consumer(&context_one, prefix, &durable).await?;

    let publish_started = Instant::now();
    let mut publisher = store_one.clone();
    for ordinal in 0..BACKLOG_EVENTS as u64 {
        publisher
            .publish_to_automation(Uuid::now_v7(), ViewEvent::Applied(ordinal))
            .await?;
    }
    let publish_elapsed = publish_started.elapsed();

    let second = start_view(store_two, projector, durable.clone());
    wait_for_applied(&state, BACKLOG_EVENTS).await?;
    wait_for_ack_floor(&context_one, prefix, &durable, BACKLOG_EVENTS as u64).await?;

    let applied = state.applied();
    let sequences = applied
        .iter()
        .map(|(_, sequence)| *sequence)
        .collect::<Vec<_>>();
    let strictly_increasing = sequences.windows(2).all(|pair| pair[0] < pair[1]);
    let mut latencies = state
        .latencies
        .lock()
        .expect("latency lock should not be poisoned")
        .clone();
    latencies.sort_unstable();
    let mut consumer = context_one
        .get_stream(prefix)
        .await?
        .get_consumer::<ConsumerConfig>(&durable)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let info = consumer.info().await?.clone();

    stop_views([first, second]).await;
    context_one.delete_stream(prefix).await?;

    println!(
        "fixture_events={BACKLOG_EVENTS} clients=2 active_consumers=1 max_ack_pending={} max_in_flight={} attempts={} effects={} order={} errors=0 timeouts=0 publish_duration_ms={} project_p50_us={} project_p95_us={} project_p99_us={} cleanup=PASS",
        info.config.max_ack_pending,
        state.max_in_flight.load(Ordering::SeqCst),
        state.attempts.load(Ordering::SeqCst),
        applied.len(),
        if strictly_increasing { "STRICT" } else { "VIOLATED" },
        publish_elapsed.as_millis(),
        percentile(&latencies, 50).as_micros(),
        percentile(&latencies, 95).as_micros(),
        percentile(&latencies, 99).as_micros(),
    );

    anyhow::ensure!(
        state.max_in_flight.load(Ordering::SeqCst) == 1,
        "multiple live-view projector calls overlapped"
    );
    anyhow::ensure!(applied.len() == BACKLOG_EVENTS, "view effect count changed");
    anyhow::ensure!(
        strictly_increasing,
        "view effects completed out of stream order"
    );
    anyhow::ensure!(
        state.attempts.load(Ordering::SeqCst) == BACKLOG_EVENTS,
        "view events were unexpectedly redelivered"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn failed_view_event_blocks_later_events_until_redelivery_succeeds() -> Result<()> {
    let context = connect().await?;
    let prefix = leaked_prefix("MILESTONE0014BARRIER");
    let durable = format!("view-barrier-{}", Uuid::now_v7().simple());
    let store = NatsStore::try_new(context.clone(), prefix)
        .await?
        .update_durable_consumer_option(ConsumerConfig {
            ack_wait: Duration::from_millis(150),
            ..Default::default()
        });
    let state = Arc::new(ProbeState::default());
    let projector = ProbeProjector {
        state: state.clone(),
        delay: Duration::from_millis(10),
        fail_once_on: Some(1),
    };
    let runner = start_view(store.clone(), projector, durable.clone());
    wait_for_consumer(&context, prefix, &durable).await?;

    let mut publisher = store;
    publisher
        .publish_to_automation(Uuid::now_v7(), ViewEvent::Applied(1))
        .await?;
    publisher
        .publish_to_automation(Uuid::now_v7(), ViewEvent::Applied(2))
        .await?;
    wait_for_applied(&state, 2).await?;
    wait_for_ack_floor(&context, prefix, &durable, 2).await?;

    let applied = state.applied();
    stop_views([runner]).await;
    context.delete_stream(prefix).await?;

    println!(
        "fixture_events=2 clients=1 injected_failures=1 attempts={} effects={} max_in_flight={} order={} cleanup=PASS",
        state.attempts.load(Ordering::SeqCst),
        applied.len(),
        state.max_in_flight.load(Ordering::SeqCst),
        if applied.iter().map(|(value, _)| *value).eq([1, 2]) {
            "STRICT"
        } else {
            "VIOLATED"
        },
    );

    anyhow::ensure!(
        applied.iter().map(|(value, _)| *value).eq([1, 2]),
        "a later view event passed the failed event"
    );
    anyhow::ensure!(
        state.attempts.load(Ordering::SeqCst) == 3,
        "fail-once fixture did not perform exactly one redelivery"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn slow_view_projection_renews_its_acknowledgement_lease() -> Result<()> {
    let context_one = connect().await?;
    let context_two = connect().await?;
    let prefix = leaked_prefix("MILESTONE0014LEASE");
    let durable = format!("view-lease-{}", Uuid::now_v7().simple());
    let config = ConsumerConfig {
        ack_wait: Duration::from_millis(100),
        ..Default::default()
    };
    let store_one = NatsStore::try_new(context_one.clone(), prefix)
        .await?
        .update_durable_consumer_option(config.clone());
    let store_two = NatsStore::try_new(context_two, prefix)
        .await?
        .update_durable_consumer_option(config);
    let state = Arc::new(ProbeState::default());
    let projector = ProbeProjector {
        state: state.clone(),
        delay: Duration::from_millis(350),
        fail_once_on: None,
    };
    let first = start_view(store_one.clone(), projector.clone(), durable.clone());
    wait_for_consumer(&context_one, prefix, &durable).await?;
    let second = start_view(store_two, projector, durable.clone());
    wait_for_waiting_pulls(&context_one, prefix, &durable, 2).await?;

    let mut publisher = store_one;
    publisher
        .publish_to_automation(Uuid::now_v7(), ViewEvent::Applied(1))
        .await?;
    wait_for_applied(&state, 1).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let applied = state.applied();
    let attempts = state.attempts.load(Ordering::SeqCst);
    let max_in_flight = state.max_in_flight.load(Ordering::SeqCst);
    stop_views([first, second]).await;
    context_one.delete_stream(prefix).await?;

    println!(
        "fixture_events=1 clients=2 ack_wait_ms=100 projector_ms=350 attempts={attempts} effects={} max_in_flight={max_in_flight} duplicates={} cleanup=PASS",
        applied.len(),
        applied.len().saturating_sub(1),
    );

    anyhow::ensure!(attempts == 1, "slow view event was redelivered");
    anyhow::ensure!(
        applied.len() == 1,
        "slow view event produced a duplicate effect"
    );
    anyhow::ensure!(max_in_flight == 1, "slow view event executed concurrently");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn one_view_runner_retains_its_mutable_projector_state() -> Result<()> {
    let context = connect().await?;
    let prefix = leaked_prefix("MILESTONE0016STATE");
    let durable = format!("view-state-{}", Uuid::now_v7().simple());
    let store = NatsStore::try_new(context.clone(), prefix).await?;
    let projector = LocalStateProjector::default();
    let observed = projector.observed.clone();
    let changed = projector.changed.clone();
    let runner_store = store.clone();
    let runner_durable = durable.clone();
    let runner = tokio::spawn(async move {
        runner_store
            .start_view_automation(projector, &runner_durable)
            .await
    });
    wait_for_consumer(&context, prefix, &durable).await?;

    let mut publisher = store;
    for value in 1..=3 {
        publisher
            .publish_to_automation(Uuid::now_v7(), ViewEvent::Applied(value))
            .await?;
    }
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        loop {
            let notified = changed.notified();
            if observed
                .lock()
                .expect("observed lock should not be poisoned")
                .len()
                == 3
            {
                return;
            }
            notified.await;
        }
    })
    .await?;
    wait_for_ack_floor(&context, prefix, &durable, 3).await?;

    let values = observed
        .lock()
        .expect("observed lock should not be poisoned")
        .clone();
    stop_views([runner]).await;
    context.delete_stream(prefix).await?;
    println!("fixture_events=3 clients=1 local_state={values:?} expected=[1, 2, 3] cleanup=PASS");
    anyhow::ensure!(values == [1, 2, 3], "projector-local state was reset");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn different_projector_types_cannot_share_one_view_durable() -> Result<()> {
    let context = connect().await?;
    let prefix = leaked_prefix("MILESTONE0016IDENTITY");
    let durable = format!("view-identity-{}", Uuid::now_v7().simple());
    let first_store = NatsStore::try_new(context.clone(), prefix).await?;
    let second_store = NatsStore::try_new(context.clone(), prefix).await?;
    let state = Arc::new(ProbeState::default());
    let first_projector = ProbeProjector {
        state: state.clone(),
        delay: Duration::ZERO,
        fail_once_on: None,
    };
    let first = start_view(first_store, first_projector, durable.clone());
    wait_for_consumer(&context, prefix, &durable).await?;

    let result = tokio::time::timeout(
        Duration::from_millis(500),
        second_store.start_view_automation(ConflictingProjector { state }, &durable),
    )
    .await;
    first.abort();
    let _ = first.await;
    context.delete_stream(prefix).await?;
    println!(
        "same_durable=true different_projector_types=true rejected_before_binding={} cleanup=PASS",
        matches!(result, Ok(Err(_)))
    );
    anyhow::ensure!(
        matches!(result, Ok(Err(_))),
        "conflicting projector did not fail closed"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated JetStream test cluster"]
async fn matching_legacy_view_durable_is_marked_without_losing_progress() -> Result<()> {
    let context = connect().await?;
    let prefix = leaked_prefix("MILESTONE0016LEGACY");
    let durable = format!("view-legacy-{}", Uuid::now_v7().simple());
    let store = NatsStore::try_new(context.clone(), prefix).await?;
    let stream = context.get_stream(prefix).await?;
    stream
        .create_consumer(ConsumerConfig {
            durable_name: Some(durable.clone()),
            deliver_policy: DeliverPolicy::New,
            ack_policy: AckPolicy::Explicit,
            ack_wait: Duration::from_secs(30),
            max_deliver: -1,
            max_ack_pending: 1,
            filter_subjects: vec![format!("{prefix}.{}.*", ViewEvent::name())],
            ..Default::default()
        })
        .await?;

    let state = Arc::new(ProbeState::default());
    let runner = start_view(
        store.clone(),
        ProbeProjector {
            state: state.clone(),
            delay: Duration::ZERO,
            fail_once_on: None,
        },
        durable.clone(),
    );
    wait_for_waiting_pulls(&context, prefix, &durable, 1).await?;
    let mut publisher = store;
    publisher
        .publish_to_automation(Uuid::now_v7(), ViewEvent::Applied(1))
        .await?;
    wait_for_applied(&state, 1).await?;
    wait_for_ack_floor(&context, prefix, &durable, 1).await?;

    let mut consumer = context
        .get_stream(prefix)
        .await?
        .get_consumer::<ConsumerConfig>(&durable)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let info = consumer.info().await?;
    let marker = info.config.metadata.get("esrc-view-projector").cloned();
    let ack_floor = info.ack_floor.consumer_sequence;
    stop_views([runner]).await;
    context.delete_stream(prefix).await?;
    println!(
        "legacy_marker_present={} marker_matches={} ack_floor={ack_floor} effects={} cleanup=PASS",
        marker.is_some(),
        marker.as_deref() == Some(std::any::type_name::<ProbeProjector>()),
        state.applied().len()
    );
    anyhow::ensure!(
        marker.as_deref() == Some(std::any::type_name::<ProbeProjector>()),
        "legacy durable was not marked with the intended projector identity"
    );
    anyhow::ensure!(ack_floor == 1, "legacy durable progress was not retained");
    Ok(())
}

fn start_view(
    store: NatsStore,
    projector: ProbeProjector,
    durable: String,
) -> JoinHandle<esrc::error::Result<()>> {
    tokio::spawn(async move { store.start_view_automation(projector, &durable).await })
}

async fn stop_views<const N: usize>(handles: [JoinHandle<esrc::error::Result<()>>; N]) {
    for handle in &handles {
        handle.abort();
    }
    for handle in handles {
        let _ = handle.await;
    }
}

async fn wait_for_consumer(
    context: &async_nats::jetstream::Context,
    stream_name: &str,
    durable: &str,
) -> Result<()> {
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        loop {
            if let Ok(stream) = context.get_stream(stream_name).await {
                if stream.get_consumer::<ConsumerConfig>(durable).await.is_ok() {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .context("timed out waiting for durable view consumer")?
}

async fn wait_for_waiting_pulls(
    context: &async_nats::jetstream::Context,
    stream_name: &str,
    durable: &str,
    expected: usize,
) -> Result<()> {
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        loop {
            let mut consumer = context
                .get_stream(stream_name)
                .await?
                .get_consumer::<ConsumerConfig>(durable)
                .await
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            if consumer.info().await?.num_waiting >= expected {
                return Result::<()>::Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .context("timed out waiting for pull subscribers")?
}

async fn wait_for_applied(state: &ProbeState, expected: usize) -> Result<()> {
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        loop {
            let notified = state.changed.notified();
            if state.applied().len() >= expected {
                return;
            }
            notified.await;
        }
    })
    .await
    .context("timed out waiting for view effects")
}

async fn wait_for_ack_floor(
    context: &async_nats::jetstream::Context,
    stream_name: &str,
    durable: &str,
    expected: u64,
) -> Result<()> {
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        loop {
            let mut consumer = context
                .get_stream(stream_name)
                .await?
                .get_consumer::<ConsumerConfig>(durable)
                .await
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            let info = consumer.info().await?;
            if info.ack_floor.consumer_sequence >= expected && info.num_ack_pending == 0 {
                return Result::<()>::Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .context("timed out waiting for confirmed acknowledgements")?
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

fn leaked_prefix(label: &str) -> &'static str {
    Box::leak(format!("{label}{}", Uuid::now_v7().simple()).into_boxed_str())
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    let index = ((samples.len() - 1) * percentile) / 100;
    samples[index]
}
