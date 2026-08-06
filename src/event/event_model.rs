use futures::Stream;

use super::EventGroup;
use crate::envelope;
use crate::error::{self};
use crate::project::Project;

/// Default schema version for a view projector using [`ViewAutomation::start_view_automation`].
pub const DEFAULT_VIEW_PROJECTOR_VERSION: u32 = 1;

/// Stable logical identity stored on a durable view consumer.
///
/// The identifier describes the read model independently of its Rust module or type name. The
/// version should change only when resuming the same durable requires an explicit compatibility
/// or migration decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewProjectorIdentity {
    id: String,
    version: u32,
}

impl ViewProjectorIdentity {
    /// Create a stable logical projector identity.
    pub fn new(id: impl Into<String>, version: u32) -> Self {
        Self {
            id: id.into(),
            version,
        }
    }

    /// Return the stable logical projector identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the projector schema/behavior version.
    pub const fn version(&self) -> u32 {
        self.version
    }
}

/// View trait to help declare Read Models
pub mod view;

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
    async fn start_automation<P>(
        &self,
        projector: P,
        feature_name: &str,
        max_concurrency: impl Into<Option<usize>> + Send,
    ) -> error::Result<()>
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

    /// Subscribe using an explicit stable logical projector identity and version.
    async fn start_view_automation_with_identity<P>(
        &self,
        projector: P,
        feature_name: &str,
        _identity: ViewProjectorIdentity,
    ) -> error::Result<()>
    where
        P: Project + 'static,
    {
        self.start_view_automation(projector, feature_name)
    }
}

/// Special type of Automation that is executed by external events.
#[trait_variant::make(Send)]
pub trait Translation: Automation {
    /// Publish an external event to be handled by the project that is started and listening that EventGroup
    async fn publish_to_automation<E>(&mut self, id: uuid::Uuid, event: E) -> error::Result<()>
    where
        E: super::Event + crate::version::SerializeVersion;
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use futures::{stream, Stream};
    use uuid::Uuid;

    use super::{Automation, EventGroup, ViewAutomation};
    use crate::envelope::Envelope;
    use crate::event::{Event, Sequence};
    use crate::project::Project;
    use crate::version::DeserializeVersion;

    struct CompatibilityEnvelope;

    impl Envelope for CompatibilityEnvelope {
        fn id(&self) -> Uuid {
            Uuid::nil()
        }

        fn sequence(&self) -> Sequence {
            Sequence::default()
        }

        fn timestamp(&self) -> SystemTime {
            SystemTime::UNIX_EPOCH
        }

        fn name(&self) -> &str {
            "compatibility"
        }

        fn get_metadata(&self, _key: &str) -> Option<&str> {
            None
        }

        fn deserialize<E>(&self) -> crate::error::Result<E>
        where
            E: DeserializeVersion + Event,
        {
            Err(crate::error::Error::Invalid)
        }
    }

    struct HistoricalViewAutomation;

    impl Automation for HistoricalViewAutomation {
        type Envelope = CompatibilityEnvelope;

        async fn durable_subscribe<G: EventGroup>(
            &self,
            _unique_name: &str,
        ) -> crate::error::Result<impl Stream<Item = crate::error::Result<Self::Envelope>> + Send>
        {
            Ok(stream::empty())
        }

        async fn start_automation<P>(
            &self,
            _projector: P,
            _feature_name: &str,
            _max_concurrency: impl Into<Option<usize>> + Send,
        ) -> crate::error::Result<()>
        where
            P: Project + 'static,
        {
            Ok(())
        }
    }

    impl ViewAutomation for HistoricalViewAutomation {
        async fn start_view_automation<P>(
            &self,
            _projector: P,
            _feature_name: &str,
        ) -> crate::error::Result<()>
        where
            P: Project + 'static,
        {
            Ok(())
        }
    }

    #[test]
    fn historical_view_automation_impl_inherits_identity_method() {
        fn assert_compatible<T: ViewAutomation>() {}

        assert_compatible::<HistoricalViewAutomation>();
    }
}
