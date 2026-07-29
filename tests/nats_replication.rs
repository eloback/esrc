#![cfg(feature = "nats")]

use std::env;
use std::error::Error as _;
use std::time::Duration;

use anyhow::{Context as _, Result};
use async_nats::jetstream::stream::{Config as StreamConfig, DiscardPolicy, Info};
use esrc::event::{Publish, ReplayOne, Sequence};
use esrc::nats::{NatsStore, NatsStoreOptions, NatsStreamReplicaMismatch};
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Envelope, Event};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const OPERATION_TIMEOUT: Duration = Duration::from_secs(15);
const CURRENT_TIMEOUT: Duration = Duration::from_secs(30);
const FAILOVER_PREFIX: &str = "MILESTONE0004FAILOVER";
const FAILOVER_ID: Uuid = Uuid::from_u128(0x018f_0004_0000_7000_8000_0000_0000_0001);

#[derive(Debug, Deserialize, DeserializeVersion, Event, PartialEq, Serialize, SerializeVersion)]
enum ReplicatedEvent {
    Added(u64),
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires NATS_URL, NATS_USER, and NATS_PASSWORD for an isolated three-node JetStream cluster"]
async fn r3_policy_applies_to_writer_and_mirror_and_rejects_r1() -> Result<()> {
    let context = connect().await?;
    let suffix = Uuid::now_v7().simple();
    let r1_prefix: &'static str = Box::leak(format!("MILESTONE0004R1{suffix}").into_boxed_str());
    let r3_prefix: &'static str = Box::leak(format!("MILESTONE0004R3{suffix}").into_boxed_str());
    let mirror_name = format!("MILESTONE0004MIRROR{suffix}");

    context
        .create_stream(StreamConfig {
            name: r1_prefix.to_owned(),
            subjects: vec![format!("{r1_prefix}.>")],
            discard: DiscardPolicy::New,
            num_replicas: 1,
            ..Default::default()
        })
        .await?;

    let mismatch = match NatsStore::try_new_with_options(
        context.clone(),
        r1_prefix,
        NatsStoreOptions::replicated(),
    )
    .await
    {
        Ok(_) => anyhow::bail!("an existing R1 stream satisfied an R3 request"),
        Err(error) => error,
    };
    let mismatch = mismatch
        .source()
        .and_then(|source| source.downcast_ref::<NatsStreamReplicaMismatch>())
        .context("expected a typed NatsStreamReplicaMismatch source")?;
    anyhow::ensure!(mismatch.stream() == r1_prefix, "wrong mismatched stream");
    anyhow::ensure!(mismatch.expected() == 3, "wrong requested replica count");
    anyhow::ensure!(mismatch.actual() == 1, "wrong actual replica count");

    let mut r1_stream = context.get_stream(r1_prefix).await?;
    anyhow::ensure!(
        r1_stream.info().await?.config.num_replicas == 1,
        "mismatch validation mutated the existing R1 stream"
    );

    let store =
        NatsStore::try_new_with_options(context.clone(), r3_prefix, NatsStoreOptions::replicated())
            .await?;
    let writer_info = wait_until_current(&context, r3_prefix).await?;
    assert_r3_current(&writer_info)?;

    let _store = store.enable_mirror(mirror_name.clone()).await?;
    let mirror_info = wait_until_current(&context, &mirror_name).await?;
    assert_r3_current(&mirror_info)?;

    context.delete_stream(&mirror_name).await?;
    context.delete_stream(r3_prefix).await?;
    context.delete_stream(r1_prefix).await?;

    println!(
        "writer_replicas=3 writer_followers=2 mirror_replicas=3 mirror_followers=2 mismatch_expected=3 mismatch_actual=1 mismatch_mutation=NONE cleanup=PASS"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "phase-controlled fixture for setup, one-node loss, recovery, rolling restart, and cleanup"]
async fn r3_failover_fixture() -> Result<()> {
    let phase = env::var("MILESTONE0004_PHASE")
        .context("MILESTONE0004_PHASE must be setup, degraded, recovered, verify, or cleanup")?;
    let context = connect().await?;

    match phase.as_str() {
        "setup" => {
            let _ = context.delete_stream(FAILOVER_PREFIX).await;
            let mut store = NatsStore::try_new_with_options(
                context.clone(),
                FAILOVER_PREFIX,
                NatsStoreOptions::replicated(),
            )
            .await?;
            publish_values(&mut store, &[1, 2, 3]).await?;
            let info = wait_until_current(&context, FAILOVER_PREFIX).await?;
            assert_r3_current(&info)?;
            let (values, checksum) = replay_values(&store).await?;
            anyhow::ensure!(values == [1, 2, 3], "setup replay changed event order");
            print_fixture_result(&phase, &info, &values, &checksum);
        },
        "degraded" => {
            let mut store = NatsStore::try_new_with_options(
                context.clone(),
                FAILOVER_PREFIX,
                NatsStoreOptions::replicated(),
            )
            .await?;
            let (before, _) = replay_values(&store).await?;
            anyhow::ensure!(
                before == [1, 2, 3],
                "events were lost before degraded write"
            );
            publish_values(&mut store, &[4]).await?;
            let (values, checksum) = replay_values(&store).await?;
            anyhow::ensure!(
                values == [1, 2, 3, 4],
                "degraded replay contained missing, duplicate, or reordered events"
            );
            let mut stream = context.get_stream(FAILOVER_PREFIX).await?;
            let info = stream.info().await?.clone();
            print_fixture_result(&phase, &info, &values, &checksum);
        },
        "recovered" | "verify" => {
            let store = NatsStore::try_new_with_options(
                context.clone(),
                FAILOVER_PREFIX,
                NatsStoreOptions::replicated(),
            )
            .await?;
            let info = wait_until_current(&context, FAILOVER_PREFIX).await?;
            assert_r3_current(&info)?;
            let (values, checksum) = replay_values(&store).await?;
            anyhow::ensure!(
                values == [1, 2, 3, 4],
                "recovered replay contained missing, duplicate, or reordered events"
            );
            print_fixture_result(&phase, &info, &values, &checksum);
        },
        "cleanup" => {
            context.delete_stream(FAILOVER_PREFIX).await?;
            println!("phase=cleanup stream={FAILOVER_PREFIX} cleanup=PASS");
        },
        other => anyhow::bail!("unsupported MILESTONE0004_PHASE `{other}`"),
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

async fn publish_values(store: &mut NatsStore, values: &[u64]) -> Result<()> {
    let current = replay_values(store).await?.0;
    let mut sequence = if current.is_empty() {
        Sequence::new()
    } else {
        let replay = store
            .replay_one::<ReplicatedEvent>(FAILOVER_ID, Sequence::new())
            .await?;
        futures::pin_mut!(replay);
        let mut last = Sequence::new();
        while let Some(envelope) = replay.next().await {
            last = envelope?.sequence();
        }
        last
    };

    for value in values {
        sequence = tokio::time::timeout(
            OPERATION_TIMEOUT,
            store.publish(FAILOVER_ID, sequence, ReplicatedEvent::Added(*value), None),
        )
        .await
        .context("replicated publish acknowledgement timed out")??;
    }
    Ok(())
}

async fn replay_values(store: &NatsStore) -> Result<(Vec<u64>, String)> {
    tokio::time::timeout(OPERATION_TIMEOUT, async {
        let replay = store
            .replay_one::<ReplicatedEvent>(FAILOVER_ID, Sequence::new())
            .await?;
        futures::pin_mut!(replay);
        let mut values = Vec::new();
        let mut hasher = Sha256::new();
        while let Some(envelope) = replay.next().await {
            let envelope = envelope?;
            let sequence = u64::from(envelope.sequence());
            let ReplicatedEvent::Added(value) = envelope.deserialize()?;
            hasher.update(sequence.to_be_bytes());
            hasher.update(value.to_be_bytes());
            values.push(value);
        }
        Ok((values, format!("{:x}", hasher.finalize())))
    })
    .await
    .context("replicated replay timed out before end-of-stream")?
}

async fn wait_until_current(
    context: &async_nats::jetstream::Context,
    stream_name: &str,
) -> Result<Info> {
    tokio::time::timeout(CURRENT_TIMEOUT, async {
        loop {
            let mut stream = context.get_stream(stream_name).await?;
            let info = stream.info().await?.clone();
            if info.cluster.as_ref().is_some_and(|cluster| {
                cluster.leader.is_some()
                    && cluster.replicas.len() == 2
                    && cluster.replicas.iter().all(|replica| {
                        replica.current && !replica.offline && replica.lag.unwrap_or(0) == 0
                    })
            }) {
                return Result::<Info>::Ok(info);
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .context("stream did not reach one leader and two current followers")?
}

fn assert_r3_current(info: &Info) -> Result<()> {
    anyhow::ensure!(info.config.num_replicas == 3, "stream is not configured R3");
    let cluster = info
        .cluster
        .as_ref()
        .context("stream has no cluster info")?;
    anyhow::ensure!(cluster.leader.is_some(), "stream has no leader");
    anyhow::ensure!(cluster.replicas.len() == 2, "stream lacks two followers");
    anyhow::ensure!(
        cluster.replicas.iter().all(|replica| {
            replica.current && !replica.offline && replica.lag.unwrap_or(0) == 0
        }),
        "a stream follower is not current"
    );
    Ok(())
}

fn print_fixture_result(phase: &str, info: &Info, values: &[u64], checksum: &str) {
    let cluster = info.cluster.as_ref();
    let leader = cluster
        .and_then(|cluster| cluster.leader.as_deref())
        .unwrap_or("NONE");
    let followers = cluster.map_or_else(String::new, |cluster| {
        cluster
            .replicas
            .iter()
            .map(|peer| {
                format!(
                    "{}:current={}:offline={}:lag={:?}",
                    peer.name, peer.current, peer.offline, peer.lag
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    });
    println!(
        "phase={phase} stream={} replicas={} leader={leader} followers={followers} messages={} values={values:?} checksum={checksum} ack=CONFIRMED order=PASS",
        info.config.name,
        info.config.num_replicas,
        values.len(),
    );
}
