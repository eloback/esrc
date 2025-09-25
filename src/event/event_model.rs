use futures::Stream;

use super::EventGroup;
use crate::envelope;
use crate::error::{self};
use crate::project::Project;

/// Declare automations that will be executed on new events.
#[trait_variant::make(Send)]
pub trait Automation {
    /// The envelope type used by the implementing event store.
    type Envelope: envelope::Envelope;

    /// Subscribe to events published across the event streams.
    ///
    /// This includes all events identified by the EventGroup type paramter, and
    /// creates a Stream to consume these events in relative order. This method
    /// will only produce events that are published after its invocation; the
    /// Stream is infinite and will wait for new events.
    async fn durable_subscribe<G: EventGroup>(
        &self,
        unique_name: &str,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send>;

    /// Subscribe to events and project them onto the given Project type.
    ///
    /// Events published to any stream identified by the EventGroup type
    /// parameter will be included.
    async fn start_automation<P>(&self, projector: P, feature_name: &str) -> error::Result<()>
    where
        P: Project + 'static;
}

/// automation that projects events onto a read model
#[trait_variant::make(Send)]
pub trait ViewAutomation: Automation {
    /// Subscribe to events and project them onto the given Project type.
    ///
    /// Events published to any stream identified by the EventGroup type
    /// parameter will be included.
    async fn start_view_automation<P>(&self, projector: P, feature_name: &str) -> error::Result<()>
    where
        P: Project + 'static;
}

/// Special type of Automation that is executed by external events.
#[trait_variant::make(Send)]
pub trait Translation: Automation {
    /// Publish an external event to be handled by the project that is started and listening that EventGroup
    async fn publish_to_automation<E>(&mut self, id: uuid::Uuid, event: E) -> error::Result<()>
    where
        E: super::Event + crate::version::SerializeVersion;
}

/// View trait to help declare Read Models
pub mod view {
    use serde::{de::DeserializeOwned, Serialize};

    use crate::{envelope::TryFromEnvelope, event, project::Context, Envelope};

    /// tell if the state of the view changed
    pub type Changed = bool;

    /// Declare a read model that can be updated by projecting events onto it.
    #[trait_variant::make(Send)]
    pub trait View: Default + Clone + Send + Serialize + DeserializeOwned {
        /// The event(s) that can be processed by this object.
        type EventGroup: event::EventGroup + Send + TryFromEnvelope;

        /// Update the aggregate state using a previously published event.
        /// and return whether the state changed
        fn apply<'de, E>(&mut self, context: Context<'de, E, Self::EventGroup>) -> Changed
        where
            E: Envelope + Sync;
    }
}
