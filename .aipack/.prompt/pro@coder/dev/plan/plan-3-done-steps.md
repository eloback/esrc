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
