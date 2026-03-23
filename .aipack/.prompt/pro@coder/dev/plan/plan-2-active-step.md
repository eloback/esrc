## Step - Add cafe example extension and integration tests for ServiceCommandHandler
      status: active
time-created: 2026-03-23 18:55:03
time-current: 2026-03-23 19:32:56

References: see the definition in plan-3-done-steps.md,
steps 'Step - Define NatsServiceCommandHandler trait and adapter' and
'Step - Add ServiceCommandReply helper and CqrsClient dispatch method'.

Add a concrete demonstration and automated tests for the new `ServiceCommandHandler`.

- Extend `crates/esrc-cqrs/examples/cafe/`:
  - Add a `service.rs` module containing:
    - A `CafeCommands` enum with at least two variants (e.g., `PlaceOrder { ... }`,
      `CompleteOrder { id: Uuid }`).
    - A `CafeServiceHandler` struct implementing `NatsServiceCommandHandler<NatsStore, CafeCommands>`.
    - The handler routes each variant to the appropriate domain logic using the store.
  - Wire `CafeServiceHandler` into `main.rs` via `registry.register_command(ServiceCommandHandler::new(...))`.
  - Add driver client calls using `CqrsClient::dispatch_service_command` to send each variant and
    print/assert the reply.

- In `crates/esrc-cqrs/tests/integration_nats.rs`:
  - Add a section `// -- ServiceCommandHandler tests --` with:
    - `test_service_command_handler_success`: send a well-formed command through the service
      handler, assert the reply has `success: true` and the store reflects the event.
    - `test_service_command_handler_error`: send a command that triggers a domain error,
      assert the reply has `success: false` and a populated `error` field.
    - `test_service_command_handler_malformed_payload`: send garbage bytes, assert that the
      dispatcher returns an error response and remains alive for subsequent requests.
  - Reuse the existing `TestCtx`, `spawn_dispatcher`, and `CounterEvent` types.
  - Implement a `CounterServiceHandler` that wraps `CounterCommand` dispatch through the
    service handler path, so the tests exercise the new code path end-to-end.

### Implementation Considerations

- `service.rs` uses `CafeCommands` enum with `PlaceOrder` and `CompleteOrder` variants, each
  carrying the aggregate `id: Uuid` since the service handler receives the full command in one shot.
- Integration tests use a `CounterServiceHandler` wrapping the existing `Counter` aggregate and
  `CounterCommand` enum, reusing all existing test infrastructure.
- A local `esrc_to_cqrs` helper in the test file converts `esrc::error::Error` to `esrc_cqrs::Error`
  with proper `External` downcast for `CounterError`.
- The duplicate `ServiceCommandHandler` struct definition in `service_command_handler.rs` was
  removed as part of this step; the file now has a single clean definition.
- `plan-1-todo-steps.md` had this step listed; it has been moved to active.
