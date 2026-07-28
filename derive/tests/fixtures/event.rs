use esrc::event::Event;
use esrc::version::DeserializeVersion;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize, PartialEq)]
pub struct FooEvent;

#[derive(Debug, Deserialize, PartialEq)]
pub struct BarEvent;

#[derive(Debug, Deserialize, PartialEq)]
pub struct OwnedEvent {
    pub local_name: String,
}

impl Event for FooEvent {
    fn name() -> &'static str {
        "Foo"
    }
}

impl DeserializeVersion for FooEvent {
    fn deserialize_version<'de, D>(_deserializer: D, _version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(FooEvent {})
    }
}

impl Event for BarEvent {
    fn name() -> &'static str {
        "Bar"
    }
}

impl DeserializeVersion for BarEvent {
    fn deserialize_version<'de, D>(_deserializer: D, _version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(BarEvent {})
    }
}

impl Event for OwnedEvent {
    fn name() -> &'static str {
        "Owned"
    }
}

impl DeserializeVersion for OwnedEvent {
    fn deserialize_version<'de, D>(_deserializer: D, _version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(OwnedEvent {
            local_name: "LocalOwned".to_owned(),
        })
    }
}
