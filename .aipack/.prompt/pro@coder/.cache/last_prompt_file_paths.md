
====
file_content_mode: udiffx

# Context Files

## All resolve files(sent to AI with content, in this order)

- examples/cafe/error.rs
- examples/cafe/tab.rs
- src/error.rs
- src/nats/convert.rs
- src/kurrent/convert.rs
- src/kurrent/subject.rs
- src/kurrent.rs
- src/kurrent/header.rs
- src/version.rs
- src/nats/subject.rs
- src/kurrent/event.rs
- src/envelope.rs
- src/kurrent/envelope.rs
- src/nats/header.rs
- src/project.rs
- src/event.rs
- src/nats.rs
- crates/esrc-cqrs/src/command.rs
- crates/esrc-cqrs/src/nats/projector_runner.rs
- crates/esrc-cqrs/src/projector.rs
- examples/cafe/projector.rs
- crates/esrc-cqrs/src/nats/command_dispatcher.rs
- crates/esrc-cqrs/src/nats/aggregate_command_handler.rs
- crates/esrc-cqrs/src/error.rs
- src/nats/event.rs
- src/nats/envelope.rs
- crates/esrc-cqrs/src/lib.rs
- crates/esrc-cqrs/src/query.rs
- crates/esrc-cqrs/src/registry.rs
- src/aggregate.rs
- crates/esrc-cqrs/src/nats/query_dispatcher.rs
- src/lib.rs
- src/view.rs
- crates/esrc-cqrs/src/nats/memory_view_query.rs
- examples/cafe/domain.rs
- crates/esrc-cqrs/src/nats/live_view_query.rs
- examples/cafe/table.rs
- crates/esrc-cqrs/src/nats/durable_projector_handler.rs
- crates/esrc-cqrs/src/nats/mod.rs
- examples/cafe/main.rs
- crates/esrc-cqrs/Cargo.toml
- Cargo.toml

# Working Files

None

# Structure Files

(Only file paths, not their content)

- crates/esrc-cqrs/
  - Cargo.toml
- crates/esrc-cqrs/src/
  - command.rs
  - error.rs
  - lib.rs
  - projector.rs
  - query.rs
  - registry.rs
- crates/esrc-cqrs/src/nats/
  - aggregate_command_handler.rs
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
- examples/cafe/
  - domain.rs
  - error.rs
  - main.rs
  - projector.rs
  - tab.rs
  - table.rs
- examples/cafe/tab/
  - tests.rs
- examples/zero_copy/
  - main.rs
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

