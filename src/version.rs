#[cfg(feature = "derive")]
#[doc(inline)]
pub use esrc_derive::{DeserializeVersion, SerializeVersion};
use serde::{Deserialize, Deserializer, Serialize};

/// Extend the serde Deserialize trait with extra versioning info.
pub trait DeserializeVersion<'de>: Deserialize<'de> {
    /// Deserialize the implementing type, using the specified version.
    ///
    /// In the context of this library, a version is implicitly stored with all
    /// serialized types (that use [`SerializeVersion`]). When deserialized,
    /// this version can be used to upcast events, etc.
    fn deserialize_version<D>(deserializer: D, version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

/// Extend the serde Serialize trait with extra versioning info.
pub trait SerializeVersion: Serialize {
    /// Specify a version to be encoded alongside the serialized type.
    ///
    /// Where this version info is stored depends on the event store backend.
    fn version() -> usize;
}
