#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Materialize and mutate Aggregates on top of event streams.
pub mod aggregate;
/// Generic event wrappers that can be deserialized safely into Event types.
pub mod envelope;
/// Common error types used throughout the esrc modules.
pub mod error;
/// Traits and helpers for the core Event type and event store implementations.
pub mod event;
/// Process events and perform side effects for Events outside of an Aggregate.
pub mod project;
/// (De)Serialize types with extra version information for upcasting.
pub mod version;

/// An event store implementation on top of NATS Jetstream.
#[cfg(feature = "nats")]
pub mod nats;

pub use aggregate::Aggregate;
pub use envelope::Envelope;
pub use error::Error;
pub use event::{Event, EventGroup};
