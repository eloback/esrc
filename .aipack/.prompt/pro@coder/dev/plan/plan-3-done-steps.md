## Step - Fix cargo check errors and warnings in esrc-cqrs
      status: done
time-created: 2026-03-19 19:23:51
   time-done: 2026-03-19 20:02:40

Fixed all errors and warnings reported by `cargo check`:

- **`crates/esrc-cqrs/Cargo.toml`**: added `uuid` with the `serde` feature.
- **`crates/esrc-cqrs/src/nats/aggregate_command_handler.rs`**: removed unused imports.
- **`crates/esrc-cqrs/src/nats/aggregate_projector_handler.rs`**: removed unused imports; implemented `durable_observe` as an inherent method on `NatsStore` in `src/nats/event.rs` and called it from `DurableProjectorHandler`.
- **`crates/esrc-cqrs/src/nats/command_dispatcher.rs`**: removed unused import `esrc::nats::NatsStore`.
- **`crates/esrc-cqrs/src/registry.rs`**: added `+ Sync` bound to `register_projector`'s `H` type parameter.

## Step - Create the cafe example skeleton with domain types
      status: done
time-created: 2026-03-20 11:53:33
   time-done: 2026-03-20 12:02:53

Created the `examples/cafe/` directory with core domain types for a cafe ordering system:

- `examples/cafe/domain.rs`: `Order` aggregate, `OrderCommand` enum, `OrderEvent` enum
  (deriving `Event`, `SerializeVersion`, `DeserializeVersion`, `Serialize`, `Deserialize`),
  `OrderStatus` enum, `OrderError`, and `impl Aggregate for Order`.
- `examples/cafe/main.rs`: entry point stub.
- `Cargo.toml`: `[[example]]` entry for `cafe` pointing to `examples/cafe/main.rs`
  with `required-features = ["nats", "derive"]`.

## Step - Implement command handlers and projector for the cafe example
      status: done
time-created: 2026-03-20 11:53:33
   time-done: 2026-03-20 12:02:53

Wired up the CQRS layer and completed the cafe example:

- `examples/cafe/projector.rs`: `OrderProjector` implementing `esrc::project::Project` for
  `OrderEvent`, printing order activity to stdout.
- `examples/cafe/main.rs` (full implementation):
  - Connects to NATS, builds `NatsStore`, `CqrsRegistry`, `NatsCommandDispatcher`.
  - Registers `AggregateCommandHandler::<Order>::new("Order")` and
    `DurableProjectorHandler::new("cafe-orders", OrderProjector::default())`.
  - Spawns projectors via `registry.run_projectors()`.
  - Spawns a client driver task that sends `PlaceOrder` and `CompleteOrder` commands
    via `async_nats::Client::request` and prints the `CommandReply`.
  - Runs the command dispatcher (blocks until NATS closes).

