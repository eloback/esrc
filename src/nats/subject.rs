use std::borrow::Cow;

use uuid::Uuid;

use crate::error::{self, Error};

pub enum NatsSubject<'a> {
    Wildcard,
    Event(Cow<'a, str>),
    Aggregate(Cow<'a, str>, Uuid),
}

impl NatsSubject<'_> {
    pub fn try_from_str(expected_prefix: &str, subject: &str) -> error::Result<Self> {
        let mut parts = subject.split('.').fuse();

        let prefix = parts.next().ok_or(Error::Invalid)?;
        if prefix != expected_prefix {
            return Err(Error::Invalid);
        }

        let name = parts.next();
        let id = parts.next();

        if let (Some(name), Some(id)) = (name, id) {
            let parsed_id = Uuid::try_parse(id).map_err(|e| Error::Format(e.into()))?;
            Ok(Self::Aggregate(name.to_owned().into(), parsed_id))
        } else if let Some(name) = name {
            Ok(Self::Event(name.to_owned().into()))
        } else {
            Ok(Self::Wildcard)
        }
    }

    pub fn into_string(self, prefix: &str) -> String {
        match self {
            Self::Wildcard => format!("{prefix}.>"),
            Self::Event(name) => format!("{prefix}.{name}.*"),
            Self::Aggregate(name, id) => format!("{prefix}.{name}.{id}"),
        }
    }
}
