use async_nats::jetstream;
use esrc::aggregate::Root;
use esrc::event::{PublishExt, SubscribeExt};
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Aggregate, Envelope, Event};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

// Events can take advantage of serde's zero-copy deserialization, where
// lifetimes in an event will be tied to the lifetime of the source Envelope.
// The exact support for this depends on the serialization format of the event
// store backend (for example, the NATS backend uses JSON).
#[derive(Event, Deserialize, DeserializeVersion, Serialize, SerializeVersion)]
#[esrc(serde(version = 1))]
enum ZeroCopyEvent<'a> {
    Created(&'a str),
    Destroyed,
}

#[derive(Clone)]
struct NamePrinter;

#[derive(Debug, thiserror::Error)]
enum NamePrinterError {}

// The lifetime of the Project trait refers to the lifetime of the source
// Envelope; when used with the Subscribe extensions, this is an HRTB lifetime
// as the Envelope only exists within a single subscription iteration.
impl<'a> Project<'a> for NamePrinter {
    type EventGroup = ZeroCopyEvent<'a>;
    type Error = NamePrinterError;

    async fn project<E>(
        &mut self,
        context: Context<'a, E, Self::EventGroup>,
    ) -> Result<(), Self::Error>
    where
        E: Envelope + Sync,
    {
        match *context {
            ZeroCopyEvent::Created(name) => println!("Created: {}", name),
            ZeroCopyEvent::Destroyed => println!("Destroyed"),
        }

        Ok(())
    }
}

#[derive(Default)]
struct DemoAggregate {
    name: Option<String>,
    active: bool,
}

enum DemoCommand {
    Create(String),
    Destroy,
}

#[derive(Debug, thiserror::Error)]
enum DemoError {
    #[error("already exists")]
    AlreadyExists,
    #[error("not found")]
    NotFound,
}

impl Aggregate for DemoAggregate {
    type Command = DemoCommand;
    type Event = ZeroCopyEvent<'static>;
    type Error = DemoError;

    fn process(&self, command: Self::Command) -> Result<Self::Event, Self::Error> {
        match command {
            DemoCommand::Create(name) => {
                if self.active {
                    Err(DemoError::AlreadyExists)
                } else {
                    Ok(ZeroCopyEvent::Created(Box::leak(name.into_boxed_str())))
                }
            },
            DemoCommand::Destroy => {
                if !self.active {
                    Err(DemoError::NotFound)
                } else {
                    Ok(ZeroCopyEvent::Destroyed)
                }
            },
        }
    }

    fn apply(mut self, event: &Self::Event) -> Self {
        match event {
            ZeroCopyEvent::Created(name) => {
                self.name = Some(name.to_string());
                self.active = true;
            },
            ZeroCopyEvent::Destroyed => {
                self.active = false;
            },
        }
        self
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = async_nats::connect("localhost").await?;
    let context = jetstream::new(client);

    let store = NatsStore::try_new(context, "zero-copy").await?;

    let projector = NamePrinter;
    let store_clone = store.clone();

    // Start observing in a background task
    tokio::spawn(async move {
        if let Err(e) = store_clone.observe(projector).await {
            eprintln!("Observer error: {}", e);
        }
    });

    // Give the observer time to start
    sleep(Duration::from_millis(100)).await;

    println!("Starting zero-copy example...");

    let mut store_mut = store.clone();

    // Create some demo aggregates and publish events
    let id1 = Uuid::now_v7();
    let id2 = Uuid::now_v7();

    let root1 = Root::<DemoAggregate>::new(id1);
    let root2 = Root::<DemoAggregate>::new(id2);

    println!("Creating entities...");

    let _root1 = store_mut
        .try_write(root1, DemoCommand::Create("Alice".to_string()))
        .await?;
    let root2 = store_mut
        .try_write(root2, DemoCommand::Create("Bob".to_string()))
        .await?;

    // Wait for events to be processed
    sleep(Duration::from_millis(500)).await;

    println!("Destroying one entity...");
    store_mut.try_write(root2, DemoCommand::Destroy).await?;

    // Wait for final processing
    sleep(Duration::from_millis(500)).await;

    println!("Zero-copy example completed!");

    Ok(())
}
