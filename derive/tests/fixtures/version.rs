use serde::de::value::{self, UnitDeserializer};
use serde::Deserializer;

#[allow(dead_code)]
pub fn unit_deserializer<'de>() -> impl Deserializer<'de> {
    UnitDeserializer::<value::Error>::new()
}
