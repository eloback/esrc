use std::collections::HashMap;
use std::fmt::Write as _;

use async_nats::{HeaderMap, HeaderValue, Message};
use sha2::{Digest, Sha256};

pub const VERSION_KEY: &str = "Esrc-Version";
pub const EVENT_TYPE: &str = "Esrc-Event-Type";
pub const METADATA_PREFIX: &str = "xn-";

pub fn new() -> HeaderMap {
    #[cfg(feature = "opentelemetry")]
    {
        opentelemetry_nats::NatsHeaderInjector::default_with_span().into()
    }

    #[cfg(not(feature = "opentelemetry"))]
    {
        HeaderMap::new()
    }
}

pub fn event_message_id(
    subject: &str,
    last_sequence: u64,
    version: usize,
    event_type: &str,
    payload: &[u8],
    metadata: Option<&HashMap<String, String>>,
) -> String {
    let mut hasher = Sha256::new();
    update_hash(&mut hasher, b"esrc-nats-occ-event-v1");
    update_hash(&mut hasher, subject.as_bytes());
    update_hash(&mut hasher, &last_sequence.to_be_bytes());
    update_hash(
        &mut hasher,
        &u64::try_from(version)
            .expect("event version must fit in a u64")
            .to_be_bytes(),
    );
    update_hash(&mut hasher, event_type.as_bytes());
    update_hash(&mut hasher, payload);

    let mut metadata = metadata
        .into_iter()
        .flat_map(HashMap::iter)
        .collect::<Vec<_>>();
    metadata.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    update_hash(
        &mut hasher,
        &u64::try_from(metadata.len())
            .expect("metadata count must fit in a u64")
            .to_be_bytes(),
    );
    for (key, value) in metadata {
        update_hash(&mut hasher, key.as_bytes());
        update_hash(&mut hasher, value.as_bytes());
    }

    let digest = hasher.finalize();
    let mut message_id = String::with_capacity(72);
    message_id.push_str("esrc-v1-");
    for byte in digest {
        write!(&mut message_id, "{byte:02x}").expect("writing to a String cannot fail");
    }
    message_id
}

fn update_hash(hasher: &mut Sha256, value: &[u8]) {
    hasher.update(
        u64::try_from(value.len())
            .expect("hash input length must fit in a u64")
            .to_be_bytes(),
    );
    hasher.update(value);
}

pub fn get<'a>(message: &'a Message, key: &str) -> Option<&'a str> {
    message
        .headers
        .as_ref()
        .and_then(|headers| headers.get(key))
        .map(HeaderValue::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_message_id_is_stable_across_metadata_insertion_order() {
        let mut first = HashMap::new();
        first.insert("zeta".to_owned(), "last".to_owned());
        first.insert("alpha".to_owned(), "first".to_owned());
        let mut second = HashMap::new();
        second.insert("alpha".to_owned(), "first".to_owned());
        second.insert("zeta".to_owned(), "last".to_owned());

        let first_id = event_message_id("EVENTS.Counter.id", 4, 1, "Added", b"7", Some(&first));
        let second_id = event_message_id("EVENTS.Counter.id", 4, 1, "Added", b"7", Some(&second));

        assert_eq!(first_id, second_id);
    }

    #[test]
    fn event_message_id_changes_with_event_or_position() {
        let baseline = event_message_id("EVENTS.Counter.id", 4, 1, "Added", b"7", None);
        let different_payload = event_message_id("EVENTS.Counter.id", 4, 1, "Added", b"8", None);
        let different_position = event_message_id("EVENTS.Counter.id", 5, 1, "Added", b"7", None);

        assert_ne!(baseline, different_payload);
        assert_ne!(baseline, different_position);
    }
}
