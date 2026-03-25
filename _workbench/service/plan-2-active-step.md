## Step - Implement blocking serve on NatsStore
      status: active
time-created: 2026-03-25 15:39:07
time-current: 2026-03-25 17:54:46

- Create `src/nats/command_service.rs` and implement `CommandService::serve` for `NatsStore`.
- The endpoint subject is derived from `A::Event::name()`, using the wildcard pattern `<event_name>.*` to capture the aggregate UUID from the last subject token.
- On each request: parse the UUID from the subject, call `read` (`ReplayOneExt`) to load `Root<A>` from sequence 0, deserialize the command from the request payload, call `try_write` (`PublishExt`), reply with an empty body on success or a JSON-serialized `CommandError` with a NATS error status on failure.
- `Aggregate::Command` must implement `serde::de::DeserializeOwned`; enforce this bound at the call site only.
- Wire the new module into `src/nats.rs`.

### Implementation Notes

- Added `status_code()` method to `CommandError` for mapping error kinds to HTTP-like status codes used by the NATS service error API.
- The NATS service `respond(Err(...))` does not carry a custom body payload; the structured `CommandError` is logged for observability. The caller receives the error description and status code through the NATS service error mechanism.
- The `serve` method clones `NatsStore` for each request to obtain a mutable handle for `try_write`, since `NatsStore` is `Clone`.
- Service and group names are derived from `A::Event::name()`, endpoint is named `"command"`, so the full subject pattern becomes `<event_name>.<event_name>.command.<uuid>`.
