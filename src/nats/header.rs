use async_nats::{HeaderValue, Message};

pub const VERSION_KEY: &str = "Esrc-Version";
pub const EVENT_TYPE: &str = "Esrc-Event-Type";

pub fn get<'a>(message: &'a Message, key: &str) -> Option<&'a str> {
    message
        .headers
        .as_ref()
        .and_then(|headers| headers.get(key))
        .map(HeaderValue::as_str)
}
