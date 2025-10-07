use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kurrentdb::ResolvedEvent;
use serde_json::Deserializer;
use tracing::instrument;
use uuid::Uuid;

use crate::envelope::Envelope;
use crate::error::{self, Error};
use crate::event::{Event, Sequence};
use crate::kurrent::header::VERSION_KEY;
use crate::kurrent::subject::KurrentSubject;
use crate::version::DeserializeVersion;

/// Hold information needed to parse event types from a kurrentdb event.
///
/// Fields derived from the subject name and various other headers will be parsed
/// upon creation and stored alongside the original event.
pub struct KurrentEnvelope {
    id: Uuid,
    sequence: u64,

    timestamp: i64,

    name: String,
    version: usize,
    resolved_event: ResolvedEvent,
}

impl KurrentEnvelope {
    /// Attempt to convert a KurrentDB event into an Envelope instance.
    ///
    /// This requires the event to:
    /// * Have an `Esrc-Version` header, which is used as the Event's version.
    #[instrument(skip_all, level = "trace")]
    pub fn try_from_message(message: ResolvedEvent) -> error::Result<Self> {
        let event = message.get_original_event();
        let KurrentSubject::Aggregate(name, id) = KurrentSubject::try_from_str(event.stream_id())?
        else {
            return Err(Error::Invalid);
        };
        let metadata: std::collections::HashMap<String, String> =
            serde_json::from_slice(&event.custom_metadata).map_err(|e| Error::Format(e.into()))?;
        let version = metadata
            .get(VERSION_KEY)
            .ok_or(Error::Invalid)?
            .parse::<usize>()
            .map_err(|e| Error::Format(e.into()))?;

        let sequence = event.position.commit;
        let timestamp = event.created.timestamp();

        Ok(Self {
            id,
            sequence,

            timestamp,

            name: name.into_owned(),
            version,
            resolved_event: message,
        })
    }
}

impl Envelope for KurrentEnvelope {
    fn id(&self) -> Uuid {
        self.id
    }

    fn sequence(&self) -> Sequence {
        self.sequence.into()
    }

    fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.timestamp as u64)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_metadata(&self, key: &str) -> Option<&str> {
        unimplemented!()
    }

    #[instrument(skip_all, level = "trace")]
    fn deserialize<E>(&self) -> error::Result<E>
    where
        E: DeserializeVersion + Event,
    {
        if self.name != E::name() {
            return Err(Error::Invalid);
        }

        let mut deserializer =
            Deserializer::from_slice(&self.resolved_event.get_original_event().data);
        E::deserialize_version(&mut deserializer, self.version).map_err(|e| Error::Format(e.into()))
    }
}
