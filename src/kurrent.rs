use kurrentdb::Client;

use tracing::instrument;

use crate::error;

#[doc(hidden)]
pub mod convert;

/// Wrapper for Events from kurrentdb as an esrc Envelope.
pub mod envelope;

#[doc(hidden)]
pub mod event;

mod header;
mod subject;

/// A handle to an event store implementation on top of KurrentDB.
#[derive(Clone)]
pub struct KurrentStore {
    client: Client,
}

impl KurrentStore {
    /// Create a new instance of a KurrentDB event store.
    #[instrument(skip_all, level = "debug")]
    pub async fn try_new(client: Client) -> error::Result<Self> {
        Ok(Self { client })
    }
}
