## Step - Add ServiceCommandReply helper and CqrsClient dispatch method
      status: active
time-created: 2026-03-23 18:55:03
time-current: 2026-03-23 19:08:14

References: see the definition in plan-3-done-steps.md,
step 'Step - Define NatsServiceCommandHandler trait and adapter'.

- In `crates/esrc-cqrs/src/nats/command/service_command_handler.rs`:
  - Add `ServiceCommandReply<R>` struct (generic over `R: Serialize + DeserializeOwned`) with
    fields `success: bool`, `data: Option<R>`, `error: Option<crate::Error>`.
  - Derive `Serialize`, `Deserialize`, `Debug`.
  - This is an opt-in convenience type; the user may use it or return any other serializable
    bytes from their `handle` implementation.
  - Add a `ServiceCommandReply::<()>::ok()` constructor and `ServiceCommandReply::err(e)` constructor.

- In `crates/esrc-cqrs/src/nats/client/cqrs_client.rs`:
  - Add method `dispatch_service_command<C, R>(&self, service_name, command_name, command: C) -> Result<R>`
    where `C: Serialize`, `R: DeserializeOwned`.
    - Sends the serialized command to subject `<service_name>.<command_name>`.
    - Deserializes the raw reply bytes as `R` and returns it.
  - This is intentionally low-level: the user chooses `R` to match whatever their handler returns.

- Update `crates/esrc-cqrs/src/lib.rs` re-exports to expose `ServiceCommandReply`.

### Implementation Considerations

- `ServiceCommandReply<()>` gets a dedicated `ok()` constructor (no data), while the generic
  `ok_with(data: R)` constructor covers cases where data needs to be returned.
- `send_service_command` is the low-level raw variant; `dispatch_service_command` unwraps the
  `ServiceCommandReply` envelope and returns `Result<R>` for ergonomic use.
- The `ServiceCommandHandler` struct was also created in this step since it was absent from the
  prior active step's implementation (the file existed but was empty). It now contains the full
  adapter wiring: deserialization of `C` from raw bytes, delegation to the inner handler.
