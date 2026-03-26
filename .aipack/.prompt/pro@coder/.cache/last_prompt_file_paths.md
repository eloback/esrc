
====
file_content_mode: udiffx

# Knowledge Files

## All resolve files(sent to AI with content, in this order)

- _workbench/queries/_plan-rules.md

## Pre (just for info, included in the All resolved files above)

- _workbench/queries/_plan-rules.md

# Context Files

## All resolve files(sent to AI with content, in this order)

- src/version.rs
- src/error.rs
- src/project.rs
- src/event_modeling.rs
- src/nats.rs
- src/nats/query_service.rs
- src/query/mod.rs
- src/query/in_memory.rs
- src/lib.rs
- _workbench/queries/dev-chat.md
- _workbench/queries/plan-1-todo-steps.md
- _workbench/queries/plan-2-active-step.md
- _workbench/queries/plan-3-done-steps.md

## Post (just for info, included in the All resolved files above)

- _workbench/queries/dev-chat.md
- _workbench/queries/plan-1-todo-steps.md
- _workbench/queries/plan-2-active-step.md
- _workbench/queries/plan-3-done-steps.md

# Working Files

None

# Structure Files

(Only file paths, not their content)

- examples/basic-query-service/
  - domain.rs
  - main.rs
  - read_model.rs
- examples/cafe/
  - domain.rs
  - main.rs
- examples/multi-slice-command-service/
  - domain.rs
  - main.rs
- examples/multi-slice-command-service/queue_welcome_email/
  - mod.rs
- examples/multi-slice-command-service/send_welcome_email/
  - mod.rs
- examples/zero_copy/
  - main.rs
- src/
  - aggregate.rs
  - envelope.rs
  - error.rs
  - event.rs
  - event_modeling.rs
  - kurrent.rs
  - lib.rs
  - nats.rs
  - project.rs
  - version.rs
  - view.rs
- src/event/
  - command_service.rs
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
  - command_service.rs
  - convert.rs
  - envelope.rs
  - event.rs
  - header.rs
  - query_service.rs
  - subject.rs
- src/query/
  - in_memory.rs
  - mod.rs

