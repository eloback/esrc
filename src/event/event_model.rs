use futures::Stream;
use stream_cancel::Trigger;
use tokio::sync::oneshot::Sender;
use tokio_util::task::TaskTracker;

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

    /// Subscribe to events and project them onto the given Project type, with a way to gracefully shutdown the process.
    ///
    /// Events published to any stream identified by the EventGroup type
    /// parameter will be included.
    ///
    /// The exit_tx Sender can be used to send a Trigger that will stop the processing of new events.
    async fn start_automation_with_graceful_shutdown<P>(
        &self,
        projector: P,
        feature_name: &str,
        task_tracker: TaskTracker,
        exit_tx: Sender<Trigger>,
    ) -> error::Result<TaskTracker>
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

    /// Subscribe to events and project them onto the given Project type, with a way to gracefully shutdown the process.
    ///
    /// Events published to any stream identified by the EventGroup type
    /// parameter will be included.
    ///
    /// The exit_tx Sender can be used to send a Trigger that will stop the processing of new events.
    async fn start_view_automation_with_graceful_shutdown<P>(
        &self,
        projector: P,
        feature_name: &str,
        task_tracker: TaskTracker,
        exit_tx: Sender<Trigger>,
    ) -> error::Result<TaskTracker>
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
