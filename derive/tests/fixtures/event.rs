use esrc::event::Event;
use esrc::version::DeserializeVersion;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize, PartialEq)]
pub struct FooEvent;

#[derive(Debug, Deserialize, PartialEq)]
pub struct BarEvent;

#[derive(Debug, Deserialize, PartialEq)]
pub struct LifetimeEvent<'a> {
    pub local_name: &'a str,
}

impl Event for FooEvent {
    fn name() -> &'static str {
        "Foo"
    }
}

impl<'de> DeserializeVersion<'de> for FooEvent {
    fn deserialize_version<D>(_deserializer: D, _version: usize) -> Result<Self, D::Error>
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

impl<'de> DeserializeVersion<'de> for BarEvent {
    fn deserialize_version<D>(_deserializer: D, _version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(BarEvent {})
    }
}

impl<'a> Event for LifetimeEvent<'a> {
    fn name() -> &'static str {
        "Lifetime"
    }
}

impl<'a, 'de: 'a> DeserializeVersion<'de> for LifetimeEvent<'a> {
    fn deserialize_version<D>(_deserializer: D, _version: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(LifetimeEvent {
            local_name: "LocalLifetime",
        })
    }
}
