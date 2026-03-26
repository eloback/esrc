use std::future::Future;

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error,
    version::{DeserializeVersion, SerializeVersion},
};

/// Serve aggregates as command-handling service endpoints.
///
/// Implementations are responsible for receiving serialized commands,
/// loading the targeted aggregate from the event store, invoking aggregate
/// command handling, persisting any produced event, and returning a reply to
/// the caller that indicates success or failure.
#[trait_variant::make(Send)]
pub trait CommandService {
    /// Start serving commands for the given aggregate type.
    ///
    /// Implementations typically keep running until the underlying transport
    /// is closed or an unrecoverable error occurs.
    ///
    /// The exact transport mapping is implementation specific. For example, a
    /// backend may derive an endpoint subject from `A::Event::name()`, decode
    /// incoming command payloads into `A::Command`, reconstruct the aggregate
    /// through replay, and then call into aggregate command handling before
    /// replying to the requester.
    fn serve<A>(&self) -> impl Future<Output = error::Result<()>> + Send
    where
        A: crate::aggregate::Aggregate + Send + Sync + 'static,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: DeserializeOwned + Send,
        A::Error: std::error::Error + Serialize + DeserializeOwned + Send + Sync + 'static;
}

#[trait_variant::make(Send)]
/// Send commands to aggregate command-handling service endpoints.
///
/// Implementations are responsible for serializing aggregate commands,
/// routing them to the appropriate transport endpoint for aggregate `A`,
/// awaiting the service reply, and mapping transport or service failures
/// back into [`error::Error`].
pub trait CommandClient {
    /// Send a command for an aggregate instance and await the response.
    ///
    /// Implementations serialize `command`, route it to the service endpoint
    /// associated with aggregate `A` and aggregate identifier `id`, then wait
    /// for the service reply.
    ///
    /// On success this returns `Ok(())`. Transport failures, serialization
    /// issues, optimistic concurrency conflicts, and aggregate-defined command
    /// errors are returned as [`error::Error`].
    async fn send_command<A>(&self, id: uuid::Uuid, command: A::Command) -> error::Result<()>
    where
        A: crate::aggregate::Aggregate + Send + Sync + 'static,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: serde::ser::Serialize + Send,
        A::Error: std::error::Error + Serialize + DeserializeOwned + Send + Sync + 'static;
}
