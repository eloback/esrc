
====
file_content_mode: udiffx

# Context Files

## All resolve files(sent to AI with content, in this order)

- Cargo.toml
- src/aggregate.rs
- src/envelope.rs
- src/error.rs
- src/event.rs
- src/event/future.rs
- src/event/publish.rs
- src/event/replay.rs
- src/event/subscribe.rs
- src/event/truncate.rs
- src/kurrent.rs
- src/kurrent/convert.rs
- src/kurrent/envelope.rs
- src/kurrent/event.rs
- src/kurrent/header.rs
- src/kurrent/subject.rs
- src/lib.rs
- src/nats.rs
- src/nats/convert.rs
- src/nats/envelope.rs
- src/nats/event.rs
- src/nats/header.rs
- src/nats/subject.rs
- src/project.rs
- src/version.rs
- src/view.rs
- crates/esrc-cqrs/.devenv/bootstrap/bootstrapLib.nix
- crates/esrc-cqrs/.devenv/bootstrap/default.nix
- crates/esrc-cqrs/.devenv/bootstrap/resolve-lock.nix
- crates/esrc-cqrs/.devenv/nixpkgs-config-c7c9559ef5bdea25.nix
- crates/esrc-cqrs/.gitignore
- crates/esrc-cqrs/Cargo.toml
- crates/esrc-cqrs/examples/cafe/domain.rs
- crates/esrc-cqrs/examples/cafe/main.rs
- crates/esrc-cqrs/examples/cafe/projector.rs
- crates/esrc-cqrs/examples/cafe/service.rs
- crates/esrc-cqrs/src/command.rs
- crates/esrc-cqrs/src/error.rs
- crates/esrc-cqrs/src/lib.rs
- crates/esrc-cqrs/src/nats/client/cqrs_client.rs
- crates/esrc-cqrs/src/nats/client/mod.rs
- crates/esrc-cqrs/src/nats/command/aggregate_command_handler.rs
- crates/esrc-cqrs/src/nats/command/mod.rs
- crates/esrc-cqrs/src/nats/command/service_command_handler.rs
- crates/esrc-cqrs/src/nats/command_dispatcher.rs
- crates/esrc-cqrs/src/nats/durable_projector_handler.rs
- crates/esrc-cqrs/src/nats/mod.rs
- crates/esrc-cqrs/src/nats/projector_runner.rs
- crates/esrc-cqrs/src/nats/query/live_view_query.rs
- crates/esrc-cqrs/src/nats/query/memory_view_query.rs
- crates/esrc-cqrs/src/nats/query/mod.rs
- crates/esrc-cqrs/src/nats/query_dispatcher.rs
- crates/esrc-cqrs/src/projector.rs
- crates/esrc-cqrs/src/query.rs
- crates/esrc-cqrs/src/registry.rs
- crates/esrc-cqrs/tests/integration_nats.rs
- .aipack/.prompt/pro@coder/dev/plan/_plan-rules.md
- .aipack/.prompt/pro@coder/dev/plan/plan-1-todo-steps.md
- .aipack/.prompt/pro@coder/dev/plan/plan-2-active-step.md
- .aipack/.prompt/pro@coder/dev/plan/plan-3-done-steps.md

## Post (just for info, included in the All resolved files above)

- .aipack/.prompt/pro@coder/dev/plan/_plan-rules.md
- .aipack/.prompt/pro@coder/dev/plan/plan-1-todo-steps.md
- .aipack/.prompt/pro@coder/dev/plan/plan-2-active-step.md
- .aipack/.prompt/pro@coder/dev/plan/plan-3-done-steps.md

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
  - main.rs
  - projector.rs
  - service.rs
- crates/esrc-cqrs/src/
  - command.rs
  - error.rs
  - lib.rs
  - projector.rs
  - query.rs
  - registry.rs
- crates/esrc-cqrs/src/nats/
  - command_dispatcher.rs
  - durable_projector_handler.rs
  - mod.rs
  - projector_runner.rs
  - query_dispatcher.rs
- crates/esrc-cqrs/src/nats/client/
  - cqrs_client.rs
  - mod.rs
- crates/esrc-cqrs/src/nats/command/
  - aggregate_command_handler.rs
  - mod.rs
  - service_command_handler.rs
- crates/esrc-cqrs/src/nats/query/
  - live_view_query.rs
  - memory_view_query.rs
  - mod.rs
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

