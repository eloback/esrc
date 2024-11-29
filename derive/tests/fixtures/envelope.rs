use std::time::SystemTime;

use uuid::Uuid;

use esrc::envelope::Envelope;
use esrc::error;
use esrc::event::Sequence;
use esrc::version::DeserializeVersion;

use super::version::unit_deserializer;

pub struct EmptyEnvelope {
    name: &'static str,
}

#[allow(dead_code)]
impl EmptyEnvelope {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl Envelope for EmptyEnvelope {
    fn id(&self) -> Uuid {
        Uuid::nil()
    }

    fn sequence(&self) -> Sequence {
        Sequence::new()
    }

    fn timestamp(&self) -> SystemTime {
        SystemTime::now()
    }

    fn name(&self) -> &str {
        self.name
    }

    fn deserialize<'de, E>(&self) -> error::Result<E>
    where
        E: DeserializeVersion<'de>,
    {
        E::deserialize_version(unit_deserializer(), 1)
            .map_err(|_| error::Error::Invalid)
    }
}
