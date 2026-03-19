## Step - Fix cargo check errors and warnings in esrc-cqrs
      status: not_started
time-created: 2026-03-19 19:23:51

Fix all errors and warnings reported by `cargo check`:

- **`crates/esrc-cqrs/Cargo.toml`**: add `uuid` with the `serde` feature so that `Uuid` implements `Serialize`/`Deserialize`.

- **`crates/esrc-cqrs/src/nats/aggregate_command_handler.rs`**: remove the unused imports (`std::pin::pin`, `Error`, `IntoSendFuture`, `Context`, `futures::StreamExt`). Fix the `durable_observe` call: the method lives on the `custom` impl block inside `esrc::nats`, so import/call it correctly, or use the public `durable_observe` path on `NatsStore`.

- **`crates/esrc-cqrs/src/nats/aggregate_projector_handler.rs`**: remove unused imports (`std::pin::pin`, `Error`, `IntoSendFuture`, `Context`, `futures::StreamExt`). Fix the missing `durable_observe` method: `NatsStore::durable_observe` is defined in `src/nats/event.rs` under `pub mod custom` but is not re-exported from `esrc::nats`. Since it is a method on `NatsStore` defined in the `custom` submod (an inherent impl), it should be accessible directly on `&NatsStore` once the module is compiled. Investigate why it is not found and expose it correctly.

- **`crates/esrc-cqrs/src/nats/command_dispatcher.rs`**: remove unused import `esrc::nats::NatsStore`.

- **`crates/esrc-cqrs/src/registry.rs`**: add `+ Sync` bound to `register_projector`'s `H` type parameter to satisfy `ErasedProjectorHandler`.
