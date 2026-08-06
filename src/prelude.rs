//! Common imports for applications built with esrc.
//!
//! ```rust
//! use esrc::prelude::*;
//! ```

pub use crate::aggregate::{Aggregate, Root};
pub use crate::envelope::{Envelope, TryFromEnvelope};
pub use crate::error::Error;
pub use crate::event::event_model::{
    view::View, Automation, Translation, ViewAutomation, ViewProjectorIdentity,
    DEFAULT_VIEW_PROJECTOR_VERSION,
};
pub use crate::event::{
    Event, EventGroup, Publish, PublishExt, Replay, ReplayExt, ReplayOne, ReplayOneExt, Sequence,
    Subscribe, SubscribeExt, Truncate,
};
pub use crate::project::{Context, Project};
pub use crate::version::{DeserializeVersion, SerializeVersion};

#[cfg(feature = "nats")]
pub use crate::nats::{
    async_nats, NatsEnvelope, NatsStore, NatsStoreOptions, NatsStreamReplicaMismatch,
    NatsStreamReplicas,
};
