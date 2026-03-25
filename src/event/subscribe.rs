use std::pin::pin;

use futures::{Stream, StreamExt};

use super::future::IntoSendFuture;
use super::EventGroup;
use crate::envelope;
use crate::error::{self, Error};
use crate::project::{Context, Project};

/// Acknowledge the successful processing of an event.
#[trait_variant::make(Send)]
pub trait Acknowledge<E> {
    /// Acknowledge that the given event has been successfully processed.
    ///
    /// This method will be called after the projector has successfully processed
    /// the event, and should perform any necessary actions to prevent this event
    /// from being re-delivered in the future. The exact behavior of this method
    /// is dependent on the event store backend, but may include actions such as
    /// deleting the event from the store, or updating a cursor to skip this
    /// event in future deliveries.
    async fn acknowledge(&self, envelope: &E) -> error::Result<()>;
}

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
        P: for<'de> Project<'de>;

    /// Subscribe to events and project them onto the given Project type, with acknowledgement.
    async fn observe_with_ack<P, A>(&self, projector: P, acker: A) -> error::Result<()>
    where
        P: for<'de> Project<'de>,
        A: Acknowledge<Self::Envelope>;
}

impl<T> SubscribeExt for T
where
    T: Subscribe + Sync,
    T::Envelope: Sync,
{
    async fn observe<P>(&self, mut projector: P) -> error::Result<()>
    where
        P: for<'de> Project<'de>,
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

    async fn observe_with_ack<P, A>(&self, mut projector: P, acker: A) -> error::Result<()>
    where
        P: for<'de> Project<'de>,
        A: Acknowledge<Self::Envelope>,
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

            acker.acknowledge(&envelope).await?;
        }

        Ok(())
    }
}
