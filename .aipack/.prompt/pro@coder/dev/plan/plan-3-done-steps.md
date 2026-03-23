## Step - Define NatsServiceCommandHandler trait and adapter
      status: done
time-created: 2026-03-23 18:55:03
   time-done: 2026-03-23 19:01:15

## Step - Add ServiceCommandReply helper and CqrsClient dispatch method
      status: done
time-created: 2026-03-23 18:55:03
   time-done: 2026-03-23 19:32:56

- Added `ServiceCommandReply<R>` struct in `service_command_handler.rs` with `success`, `data`, `error` fields.
- Added `ok()`, `ok_with(data)`, and `err(e)` constructors.
- Added `send_service_command` and `dispatch_service_command` methods to `CqrsClient`.
- Fixed duplicate struct definition in `service_command_handler.rs` by removing the redundant first block.
- Re-exported `ServiceCommandReply` from `lib.rs`.
- `ServiceCommandHandler` adapter wires `NatsServiceCommandHandler` into the `CommandHandler` interface.

## Step - Add cafe example extension and integration tests for ServiceCommandHandler
      status: done
time-created: 2026-03-23 18:55:03
   time-done: 2026-03-23 19:38:50

- Added `crates/esrc-cqrs/examples/cafe/service.rs` with:
  - `CafeCommands` enum (`PlaceOrder`, `CompleteOrder`) carrying `id: Uuid` per variant.
  - `CafeServiceHandler` implementing `NatsServiceCommandHandler<NatsStore, CafeCommands>`.
  - Local `esrc_error_to_cqrs` helper converting `esrc::error::Error` to `esrc_cqrs::Error`.
- Updated `crates/esrc-cqrs/examples/cafe/main.rs`:
  - Registered `ServiceCommandHandler::new(CafeServiceHandler)` with the registry.
  - Added driver calls using `CqrsClient::dispatch_service_command` for `PlaceOrder` and `CompleteOrder`.
- Added integration tests in `crates/esrc-cqrs/tests/integration_nats.rs` under `// -- ServiceCommandHandler tests --`:
  - `test_service_command_handler_success`: verifies a well-formed service command returns `success: true` and the store reflects the event.
  - `test_service_command_handler_error`: verifies a domain-failing command returns `success: false` with a populated and dowcastable `error` field.
  - `test_service_command_handler_malformed_payload`: verifies the dispatcher survives and continues handling valid commands after receiving garbage bytes.
- Implemented `CounterServiceHandler` in the test file wrapping `CounterCommand` dispatch through the service handler path.
- Added local `esrc_to_cqrs` helper in the test file with proper `External` downcast for `CounterError`.
