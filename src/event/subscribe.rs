use std::pin::pin;

use futures::{Stream, StreamExt};
use tracing::instrument;

use super::future::IntoSendFuture;
use super::EventGroup;
use crate::envelope;
use crate::error::{self, Error};
use crate::project::{Context, Project};

/// Subscribe to new events published to a set of event streams.
#[trait_variant::make(Send)]
pub trait Subscribe {
    /// The envelope type used by the implementing event store.
    type Envelope: envelope::Envelope;

    /// Subscribe to events published across the event streams.
    ///
    /// This includes all events identified by the EventGroup type paramter, and
    /// creates a Stream to consume these events in relative order. This method
    /// will only produce events that are published after its invocation; the
    /// Stream is infinite and will wait for new events.
    async fn subscribe<G: EventGroup>(
        &self,
    ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send>;
}

/// Extensions for projecting newly published events.
#[trait_variant::make(Send)]
pub trait SubscribeExt: Subscribe {
    /// Subscribe to evens and project them onto the given Project type.
    ///
    /// Events published to any stream identified by the EventGroup type
    /// parameter will be included. Any errors raised by the projection will
    /// cause this method to stop processing future events.
    async fn observe<P>(&self, projector: P) -> error::Result<()>
    where
        P: Project;
}

impl<T> SubscribeExt for T
where
    T: Subscribe + Sync,
    T::Envelope: Sync,
{
    #[instrument(skip_all, level = "debug")]
    async fn observe<P>(&self, mut projector: P) -> error::Result<()>
    where
        P: Project,
    {
        let mut stream = pin!(self.subscribe::<P::EventGroup>().await?);
        while let Some(envelope) = stream.next().await {
            let envelope = envelope?;
            let context = Context::try_with_envelope(&envelope)?;

            projector
                .project(context)
                .into_send_future()
                .await
                .map_err(|e| Error::External(e.into()))?;
        }

        Ok(())
    }
}
