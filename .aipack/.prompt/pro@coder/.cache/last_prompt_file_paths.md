
====
file_content_mode: udiffx

# Context Files

## All resolve files(sent to AI with content, in this order)

- crates/esrc-cqrs/src/nats/aggregate_command_handler.rs
- crates/esrc-cqrs/src/nats/command_dispatcher.rs
- crates/esrc-cqrs/src/nats/durable_projector_handler.rs
- crates/esrc-cqrs/src/nats/projector_runner.rs
- crates/esrc-cqrs/src/nats/query_dispatcher.rs
- src/nats.rs
- crates/esrc-cqrs/src/nats/client.rs
- crates/esrc-cqrs/src/nats/mod.rs
- crates/esrc-cqrs/src/lib.rs
- crates/esrc-cqrs/src/nats/live_view_query.rs
- crates/esrc-cqrs/src/nats/memory_view_query.rs
- crates/esrc-cqrs/examples/cafe/main.rs

# Working Files

None

# Structure Files

(Only file paths, not their content)

- crates/esrc-cqrs/
  - .gitignore
  - Cargo.toml
- crates/esrc-cqrs/.devenv/
  - nixpkgs-config-c7c9559ef5bdea25.nix
- crates/esrc-cqrs/.devenv/bootstrap/
  - bootstrapLib.nix
  - default.nix
  - resolve-lock.nix
- crates/esrc-cqrs/examples/cafe/
  - domain.rs
  - error.rs
  - main.rs
  - projector.rs
- crates/esrc-cqrs/src/
  - command.rs
  - error.rs
  - lib.rs
  - projector.rs
  - query.rs
  - registry.rs
- crates/esrc-cqrs/src/nats/
  - aggregate_command_handler.rs
  - client.rs
  - command_dispatcher.rs
  - durable_projector_handler.rs
  - live_view_query.rs
  - memory_view_query.rs
  - mod.rs
  - projector_runner.rs
  - query_dispatcher.rs
- crates/esrc-cqrs/tests/
  - integration_nats.rs
- crates/opentelemetry-nats/
  - Cargo.toml
- crates/opentelemetry-nats/src/
  - lib.rs
- src/
  - aggregate.rs
  - envelope.rs
  - error.rs
  - event.rs
  - kurrent.rs
  - lib.rs
  - nats.rs
  - project.rs
  - version.rs
  - view.rs
- src/event/
  - future.rs
  - publish.rs
  - replay.rs
  - subscribe.rs
  - truncate.rs
- src/kurrent/
  - convert.rs
  - envelope.rs
  - event.rs
  - header.rs
  - subject.rs
- src/nats/
  - convert.rs
  - envelope.rs
  - event.rs
  - header.rs
  - subject.rs

