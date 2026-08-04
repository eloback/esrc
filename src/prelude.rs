//! Common imports for applications built with esrc.
//!
//! ```rust
//! use esrc::prelude::*;
//! ```

pub use crate::aggregate::{Aggregate, Root};
pub use crate::envelope::{Envelope, TryFromEnvelope};
pub use crate::error::Error;
pub use crate::event::event_model::{Automation, Translation, ViewAutomation};
pub use crate::event::{
    Event, EventGroup, Publish, PublishExt, Replay, ReplayExt, ReplayOne, ReplayOneExt, Sequence,
    Subscribe, SubscribeExt, Truncate,
};
pub use crate::project::{Context, Project};
pub use crate::version::{DeserializeVersion, SerializeVersion};
pub use crate::view::View;

#[cfg(feature = "nats")]
pub use crate::nats::{
    async_nats, NatsEnvelope, NatsStore, NatsStoreOptions, NatsStreamReplicaMismatch,
    NatsStreamReplicas,
};
