## Step - Diagnose and fix cafe example compilation errors
      status: not_started
time-created: 2026-03-20 12:20:57

Investigate and fix all compilation errors produced by `cargo run --example cafe`
(or `cargo check --example cafe --features nats,derive`).

The cafe example exercises the `esrc-cqrs` crate end-to-end; do **not** remove or
stub out any `esrc-cqrs` usage as a fix. Every fix must keep the crate wired up.

Scope of work:

- Run (or simulate) `cargo check --example cafe --features nats,derive` and collect
  every error and warning emitted by the compiler.

- Fix errors in any of the following files as needed:
  - `examples/cafe/main.rs`
  - `examples/cafe/domain.rs`
  - `examples/cafe/projector.rs`
  - `crates/esrc-cqrs/src/**` (if the root cause is in the crate itself)
  - `Cargo.toml` / `crates/esrc-cqrs/Cargo.toml` (missing features/deps)

- Common categories to check:
  - Missing trait imports or `use` statements in the example files.
  - Type mismatches between the domain types and the `CommandHandler` /
    `AggregateCommandHandler` generics.
  - `CommandEnvelope` / `CommandReply` field access or serde derive issues.
  - Projector `Project` impl lifetime or associated-type mismatches.
  - `NatsStore` / `CqrsRegistry` / `NatsCommandDispatcher` wiring in `main.rs`.
  - Any `unused import` or `dead_code` warnings that become errors under `#![deny]`.

- After applying fixes, the example must compile cleanly with:
  `cargo check --example cafe --features nats,derive`
