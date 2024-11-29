use async_nats::jetstream;
use serde::{Deserialize, Serialize};

use esrc::event::SubscribeExt;
use esrc::nats::NatsStore;
use esrc::project::{Context, Project};
use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc::{Envelope, Event};

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
            ZeroCopyEvent::Created(name) => println!("{}", name),
            ZeroCopyEvent::Destroyed => {},
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = async_nats::connect("localhost").await?;
    let context = jetstream::new(client);

    let store = NatsStore::try_new(context, "zero-copy").await?;

    let projector = NamePrinter;
    store.observe(projector).await?;

    Ok(())
}
