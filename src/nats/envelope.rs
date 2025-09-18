use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_nats::jetstream;
use serde_json::Deserializer;
use tracing::instrument;
use uuid::Uuid;

use super::header::{self, VERSION_KEY};
use super::subject::NatsSubject;
use crate::envelope::Envelope;
use crate::error::{self, Error};
use crate::event::{Event, Sequence};
use crate::version::DeserializeVersion;

/// Hold information needed to parse event types from a NATS Jetstream message.
///
/// Fields derived from the subject name and various NATS headers will be parsed
/// upon creation and stored alongside the original message.
///
/// When the envelope is dropped the message is automatically acked, and any error
/// is ignored, so the user may want to hold a reference of the message if they need
/// to manually ack or nack it.
pub struct NatsEnvelope {
    id: Uuid,
    sequence: u64,

    timestamp: i64,

    name: String,
    version: usize,
    message: jetstream::Message,
}

impl NatsEnvelope {
    /// Attempt to convert a NATS jetstream message into an Envelope instance.
    ///
    /// This requires the NATS message to:
    /// * Be published onto a subject matching the format
    ///   `<expected_prefix>.<name>.<uuid>`, where the name is a deserializable
    ///   Event's name, and the UUID is an aggregate ID.
    /// * Have an `Esrc-Version` header, which is used as the Event's version.
    #[instrument(skip_all, level = "trace")]
    pub fn try_from_message(
        expected_prefix: &str,
        message: jetstream::Message,
    ) -> error::Result<Self> {
        let NatsSubject::Aggregate(name, id) =
            NatsSubject::try_from_str(expected_prefix, message.subject.as_str())?
        else {
            return Err(Error::Invalid);
        };

        let version = header::get(&message, VERSION_KEY)
            .ok_or(Error::Invalid)?
            .parse::<usize>()
            .map_err(|e| Error::Format(e.into()))?;
        let (sequence, timestamp) = {
            // Parse the sequence and timestamp from the message early since
            // retrieving the messaeg info can return an error.
            let info = message.info().map_err(Error::Internal)?;
            (info.stream_sequence, info.published.unix_timestamp())
        };

        Ok(Self {
            id,
            sequence,

            timestamp,

            name: name.into_owned(),
            version,
            message,
        })
    }

    /// Attach the current OpenTelemetry span context to the message headers, if any.
    pub fn attach_span_context(&self) {
        // propagate otel span if exists
        opentelemetry_nats::attach_span_context(&self.message);
    }

    /// ack the message asynchronously, ignoring any error
    pub async fn ack(self) {
        let _ = self.message.ack().await;
    }
}

impl Envelope for NatsEnvelope {
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

    #[instrument(skip_all, level = "trace")]
    fn deserialize<E>(&self) -> error::Result<E>
    where
        E: DeserializeVersion + Event,
    {
        if self.name != E::name() {
            return Err(Error::Invalid);
        }

        let mut deserializer = Deserializer::from_slice(&self.message.payload);
        E::deserialize_version(&mut deserializer, self.version).map_err(|e| Error::Format(e.into()))
    }
}
