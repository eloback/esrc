use std::future::Future;

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error,
    version::{DeserializeVersion, SerializeVersion},
};

/// Serve an aggregate as a NATS service endpoint, processing commands.
///
/// Implementations listen for incoming command requests, deserialize the
/// command, load the aggregate via event replay, process the command,
/// publish the resulting event, and reply to the caller.
#[trait_variant::make(Send)]
pub trait CommandService {
    /// Block and serve commands for the given aggregate type.
    ///
    /// The service listens on a subject derived from `A::Event::name()` with
    /// a wildcard for the aggregate UUID (e.g. `<event_name>.*`). On each
    /// request the aggregate is loaded from sequence 0 via `read`, the
    /// command is deserialized and processed via `try_write`, and the caller
    /// receives an empty reply on success or a [`CommandError`] on failure.
    ///
    /// This method runs until the underlying transport is closed or an
    /// unrecoverable error occurs.
    fn serve<A>(&self) -> impl Future<Output = error::Result<()>> + Send
    where
        A: crate::aggregate::Aggregate + Send + Sync + 'static,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: DeserializeOwned + Send,
        A::Error: std::error::Error + Serialize + DeserializeOwned + Send + Sync + 'static;
}

#[trait_variant::make(Send)]
pub trait CommandClient {
    /// Send a command to the service and await the response.
    ///
    /// The command is serialized and sent to the subject derived from
    /// `A::Event::name()` with the aggregate UUID (e.g. `<event_name>.<id>`).
    /// The caller awaits a reply, which is empty on success or contains a
    /// [`CommandError`] on failure.
    async fn send_command<A>(&self, id: uuid::Uuid, command: A::Command) -> error::Result<()>
    where
        A: crate::aggregate::Aggregate + Send + Sync + 'static,
        A::Event: SerializeVersion + DeserializeVersion,
        A::Command: serde::ser::Serialize + Send,
        A::Error: std::error::Error + Serialize + DeserializeOwned + Send + Sync + 'static;
}
