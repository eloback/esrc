type BoxStdError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// The common error type for event-sourcing related errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred that is not related to event logic or parsing.
    /// This error is backend-specific and may refer to transport errors, etc.
    #[error("internal error ({0})")]
    Internal(#[source] BoxStdError),
    /// An error occurred while performing a user-defined action.
    ///
    /// This includes actions such as processing a
    /// [`Command`](`crate::aggregate::Aggregate::Command`) or projecting an
    /// [`Event`](`crate::event::Event`). This error may refer to any error that
    /// resulted from an action outside the core event sourcing logic.
    #[error("external error ({0})")]
    External(#[from] BoxStdError),
    /// An error occurred while deserializing an Event.
    ///
    /// The inner type will usually be from a serde implementation, but can also
    /// be a [FromStr](`std::str::FromStr`) error or another parsing error type.
    #[error("bad envelope format ({0})")]
    Format(#[source] BoxStdError),
    /// An event was parsed succesfully, but contained unexpected data.
    ///
    /// This may be caused by receiving an invalid
    /// [`Event::name`](`crate::event::Event::name`) in a stream, or an Envelope
    /// missing required metadata (like its version number).
    #[error("consumed invalid event in stream")]
    Invalid,
    /// An optimistic concurrency error occurred.
    ///
    /// This happens when trying to publish an Event with a last sequence number
    /// that is no longer the most recent sequence number. Calls in this case
    /// can usually just be retried after applying any new events.
    #[error("event transaction failed")]
    Conflict,
}

/// An alias type that always implies an event-sourcing error.
pub type Result<T> = std::result::Result<T, Error>;
