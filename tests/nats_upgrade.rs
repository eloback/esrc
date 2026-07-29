#![cfg(feature = "nats")]

use std::env;
use std::time::Duration;

use anyhow::{Context as _, Result};
use async_nats::jetstream::stream::Info;
use esrc::event::{Publish, ReplayOne, Sequence};
use esrc::nats::{NatsStore, NatsStoreOptions};
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Envelope, Event};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const OPERATION_TIMEOUT: Duration = Duration::from_secs(15);
const CURRENT_TIMEOUT: Duration = Duration::from_secs(30);
const PREFIX: &str = "MILESTONE0005UPGRADE";
const AGGREGATE_ID: Uuid = Uuid::from_u128(0x018f_0005_0000_7000_8000_0000_0000_0001);
const SEEDED_VALUES: [u64; 3] = [11, 22, 33];
const APPENDED_VALUES: [u64; 4] = [11, 22, 33, 44];

#[derive(Debug, Deserialize, DeserializeVersion, Event, PartialEq, Serialize, SerializeVersion)]
enum UpgradeEvent {
    Added(u64),
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "phase-controlled fixture for NATS Server and async-nats rolling upgrade validation"]
async fn persistent_r3_upgrade_fixture() -> Result<()> {
    let phase = env::var("MILESTONE0005_PHASE").context(
        "MILESTONE0005_PHASE must be seed, verify-seed, append, verify-appended, or cleanup",
    )?;
    let context = connect().await?;

    match phase.as_str() {
        "seed" => {
            let _ = context.delete_stream(PREFIX).await;
            let mut store = replicated_store(context.clone()).await?;
            publish_values(&mut store, &SEEDED_VALUES).await?;
            verify(&context, &store, &phase, &SEEDED_VALUES).await?;
        },
        "verify-seed" => {
            let store = replicated_store(context.clone()).await?;
            verify(&context, &store, &phase, &SEEDED_VALUES).await?;
        },
        "append" => {
            let mut store = replicated_store(context.clone()).await?;
            assert_values(&store, &SEEDED_VALUES).await?;
            publish_values(&mut store, &[44]).await?;
            verify_degraded(&context, &store, &phase, &APPENDED_VALUES).await?;
        },
        "verify-appended" => {
            let store = replicated_store(context.clone()).await?;
            verify(&context, &store, &phase, &APPENDED_VALUES).await?;
        },
        "cleanup" => {
            context.delete_stream(PREFIX).await?;
            println!("phase=cleanup stream={PREFIX} cleanup=PASS");
        },
        other => anyhow::bail!("unsupported MILESTONE0005_PHASE `{other}`"),
    }

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

async fn replicated_store(context: async_nats::jetstream::Context) -> Result<NatsStore> {
    Ok(NatsStore::try_new_with_options(context, PREFIX, NatsStoreOptions::replicated()).await?)
}

async fn publish_values(store: &mut NatsStore, values: &[u64]) -> Result<()> {
    let mut sequence = last_sequence(store).await?;
    for value in values {
        sequence = tokio::time::timeout(
            OPERATION_TIMEOUT,
            store.publish(AGGREGATE_ID, sequence, UpgradeEvent::Added(*value), None),
        )
        .await
        .context("upgrade-fixture publish acknowledgement timed out")??;
    }
    Ok(())
}

async fn last_sequence(store: &NatsStore) -> Result<Sequence> {
    let replay = store
        .replay_one::<UpgradeEvent>(AGGREGATE_ID, Sequence::new())
        .await?;
    futures::pin_mut!(replay);
    let mut last = Sequence::new();
    while let Some(envelope) = replay.next().await {
        last = envelope?.sequence();
    }
    Ok(last)
}

async fn replay_values(store: &NatsStore) -> Result<(Vec<u64>, String)> {
    let replay = tokio::time::timeout(
        OPERATION_TIMEOUT,
        store.replay_one::<UpgradeEvent>(AGGREGATE_ID, Sequence::new()),
    )
    .await
    .context("upgrade-fixture replay request timed out")??;
    futures::pin_mut!(replay);

    let mut values = Vec::new();
    let mut hasher = Sha256::new();
    while let Some(envelope) = replay.next().await {
        let envelope = envelope?;
        let sequence = u64::from(envelope.sequence());
        let UpgradeEvent::Added(value) = envelope.deserialize()?;
        hasher.update(sequence.to_be_bytes());
        hasher.update(value.to_be_bytes());
        values.push(value);
    }

    Ok((values, format!("{:x}", hasher.finalize())))
}

async fn assert_values(store: &NatsStore, expected: &[u64]) -> Result<String> {
    let (values, checksum) = replay_values(store).await?;
    anyhow::ensure!(
        values == expected,
        "upgrade fixture contained missing, duplicate, or reordered events: {values:?}"
    );
    Ok(checksum)
}

async fn verify(
    context: &async_nats::jetstream::Context,
    store: &NatsStore,
    phase: &str,
    expected: &[u64],
) -> Result<()> {
    let info = wait_until_current(context).await?;
    let checksum = assert_values(store, expected).await?;
    let cluster = info
        .cluster
        .as_ref()
        .context("stream has no cluster info")?;
    let leader = cluster.leader.as_deref().context("stream has no leader")?;
    let followers = cluster
        .replicas
        .iter()
        .map(|peer| {
            format!(
                "{}:current={}:offline={}:lag={:?}",
                peer.name, peer.current, peer.offline, peer.lag
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    println!(
        "phase={phase} stream={PREFIX} replicas={} leader={leader} followers={followers} messages={} values={expected:?} checksum={checksum} ack=CONFIRMED order=PASS",
        info.config.num_replicas,
        expected.len(),
    );
    Ok(())
}

async fn verify_degraded(
    context: &async_nats::jetstream::Context,
    store: &NatsStore,
    phase: &str,
    expected: &[u64],
) -> Result<()> {
    let mut stream = context.get_stream(PREFIX).await?;
    let info = stream.info().await?.clone();
    anyhow::ensure!(info.config.num_replicas == 3, "upgrade fixture is not R3");
    let cluster = info
        .cluster
        .as_ref()
        .context("stream has no cluster info")?;
    let leader = cluster.leader.as_deref().context("stream has no leader")?;
    anyhow::ensure!(cluster.replicas.len() == 2, "stream lacks two peer records");
    anyhow::ensure!(
        cluster.replicas.iter().filter(|peer| peer.current).count() == 1,
        "degraded stream does not have exactly one current follower"
    );
    anyhow::ensure!(
        cluster.replicas.iter().filter(|peer| peer.offline).count() == 1,
        "degraded stream does not report exactly one offline peer"
    );
    let checksum = assert_values(store, expected).await?;
    let followers = cluster
        .replicas
        .iter()
        .map(|peer| {
            format!(
                "{}:current={}:offline={}:lag={:?}",
                peer.name, peer.current, peer.offline, peer.lag
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    println!(
        "phase={phase} stream={PREFIX} replicas={} leader={leader} followers={followers} messages={} values={expected:?} checksum={checksum} ack=CONFIRMED order=PASS",
        info.config.num_replicas,
        expected.len(),
    );
    Ok(())
}

async fn wait_until_current(context: &async_nats::jetstream::Context) -> Result<Info> {
    tokio::time::timeout(CURRENT_TIMEOUT, async {
        loop {
            let mut stream = context.get_stream(PREFIX).await?;
            let info = stream.info().await?.clone();
            if info.config.num_replicas == 3
                && info.cluster.as_ref().is_some_and(|cluster| {
                    cluster.leader.is_some()
                        && cluster.replicas.len() == 2
                        && cluster
                            .replicas
                            .iter()
                            .all(|peer| peer.current && !peer.offline && peer.lag.unwrap_or(0) == 0)
                })
            {
                return Result::<Info>::Ok(info);
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .context("upgrade fixture did not reach one leader and two current followers")?
}
