use std::borrow::Cow;

use uuid::Uuid;

use crate::error::{self, Error};

pub enum KurrentSubject<'a> {
    Wildcard,
    Event(Cow<'a, str>),
    Aggregate(Cow<'a, str>, Uuid),
}

impl KurrentSubject<'_> {
    pub fn try_from_str(subject: &str) -> error::Result<Self> {
        let mut parts = subject.split('-').fuse();

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

    pub fn into_string(self) -> String {
        match self {
            Self::Wildcard => unimplemented!(),
            Self::Event(name) => format!("{name}"),
            Self::Aggregate(name, id) => format!("{name}-{id}"),
        }
    }
}
