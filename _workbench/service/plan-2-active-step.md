## Step - Implement blocking serve on NatsStore
      status: active
time-created: 2026-03-25 15:39:07
time-current: 2026-03-25 15:51:19

- Create `src/nats/command_service.rs` and implement `CommandService::serve` for `NatsStore`.
- The endpoint subject is derived from `A::Event::name()`, using the wildcard pattern `<event_name>.*` to capture the aggregate UUID from the last subject token.
- On each request: parse the UUID from the subject, call `read` (`ReplayOneExt`) to load `Root<A>` from sequence 0, deserialize the command from the request payload, call `try_write` (`PublishExt`), reply with an empty body on success or a JSON-serialized `CommandError` with a NATS error status on failure.
- `Aggregate::Command` must implement `serde::de::DeserializeOwned`; enforce this bound at the call site only.
- Wire the new module into `src/nats.rs`.

### Implementation Considerations

- Used `async_nats::service::ServiceExt` to build the NATS service from the existing client handle on `NatsStore`.
- The endpoint subject is built via `NatsSubject::Event` with an empty prefix and the leading dot stripped, yielding `<event_name>.*`.
- The `serialize_error` helper provides a safe fallback if the error struct itself cannot be serialized.
- The `#[cfg(feature = "nats")]` guard on the new module mirrors the convention used for other nats sub-modules.
- `bytes` crate is already transitively available via `async-nats`; used directly for reply payloads.
