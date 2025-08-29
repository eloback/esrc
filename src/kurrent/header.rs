use kurrentdb::RecordedEvent;

pub const VERSION_KEY: &str = "Esrc-Version";
pub const EVENT_TYPE: &str = "Esrc-Event-Type";

pub fn get<'a>(event: &'a RecordedEvent, key: &str) -> Option<&'a str> {
    event.metadata.get(key).map(String::as_str)
}
