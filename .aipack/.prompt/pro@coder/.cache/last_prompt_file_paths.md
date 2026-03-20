
====
file_content_mode: udiffx

# Context Files

## All resolve files(sent to AI with content, in this order)

- examples/cafe/error.rs
- examples/cafe/tab.rs
- src/error.rs
- examples/cafe/tab/tests.rs
- src/event/replay.rs
- src/envelope.rs
- examples/cafe/table.rs
- src/project.rs
- src/event.rs
- examples/cafe/projector.rs
- crates/esrc-cqrs/src/lib.rs
- crates/esrc-cqrs/src/query.rs
- crates/esrc-cqrs/src/registry.rs
- examples/cafe/domain.rs
- src/lib.rs
- crates/esrc-cqrs/src/nats/query_dispatcher.rs
- crates/esrc-cqrs/tests/integration_nats.rs
- examples/cafe/main.rs
- src/aggregate.rs
- crates/esrc-cqrs/src/nats/mod.rs
- crates/esrc-cqrs/src/nats/aggregate_query_handler.rs
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
  - aggregate_projector_handler.rs
  - aggregate_query_handler.rs
  - command_dispatcher.rs
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

