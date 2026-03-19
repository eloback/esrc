## Step - Fix cargo check errors and warnings in esrc-cqrs
      status: active
time-created: 2026-03-19 19:23:51
time-current: 2026-03-19 20:02:40

Fix all errors and warnings reported by `cargo check`:

- **`crates/esrc-cqrs/Cargo.toml`**: add `uuid` with the `serde` feature so that `Uuid` implements `Serialize`/`Deserialize`.

- **`crates/esrc-cqrs/src/nats/aggregate_command_handler.rs`**: remove the unused imports (`std::pin::pin`, `Error`, `IntoSendFuture`, `Context`, `futures::StreamExt`).

- **`crates/esrc-cqrs/src/nats/aggregate_projector_handler.rs`**: remove unused imports (`std::pin::pin`, `Error`, `IntoSendFuture`, `Context`, `futures::StreamExt`). Fix the missing `durable_observe` method: `NatsStore` does not expose a `durable_observe` method (only `KurrentStore` does, in `src/kurrent/event.rs`). Implement `durable_observe` as an inherent method on `NatsStore` in `src/nats/event.rs` (mirroring the pattern from `KurrentStore::durable_observe` in the `custom` block), using the existing `durable_consumer` infrastructure already on `NatsStore`. Then call it from `DurableProjectorHandler`.

- **`crates/esrc-cqrs/src/nats/command_dispatcher.rs`**: remove unused import `esrc::nats::NatsStore`.

- **`crates/esrc-cqrs/src/registry.rs`**: add `+ Sync` bound to `register_projector`'s `H` type parameter to satisfy `ErasedProjectorHandler`.
