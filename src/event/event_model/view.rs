use serde::{de::DeserializeOwned, Serialize};

use crate::{envelope::TryFromEnvelope, event, project::Context, Envelope};

/// tell if the state of the view changed
pub type Changed = bool;

/// Declare a read model that can be updated by projecting events onto it.
pub trait View: Default + Clone + Send + Serialize + DeserializeOwned {
    /// The event(s) that can be processed by this object.
    type EventGroup: event::EventGroup + Send + TryFromEnvelope;

    /// Update the aggregate state using a previously published event.
    /// and return whether the state changed
    fn apply<'de, E>(&mut self, context: Context<'de, E, Self::EventGroup>) -> Changed
    where
        E: Envelope;
}
