
====
file_content_mode: udiffx

# Knowledge Files

## All resolve files(sent to AI with content, in this order)

- _workbench/integration_between_bounded_contexts/_plan-rules.md

## Pre (just for info, included in the All resolved files above)

- _workbench/integration_between_bounded_contexts/_plan-rules.md

# Context Files

## All resolve files(sent to AI with content, in this order)

- src/error.rs
- src/event.rs
- src/nats.rs
- src/nats/command_service.rs
- _workbench/integration_between_bounded_contexts/plan-1-todo-steps.md
- _workbench/integration_between_bounded_contexts/plan-2-active-step.md

## Post (just for info, included in the All resolved files above)

- _workbench/integration_between_bounded_contexts/plan-1-todo-steps.md
- _workbench/integration_between_bounded_contexts/plan-2-active-step.md

# Working Files

None

# Structure Files

(Only file paths, not their content)

- examples/operations/
  - main.rs
- examples/operations/domain/
  - email.rs
  - mod.rs
  - operation.rs
- examples/operations/kv_operation_view/
  - mod.rs
- examples/operations/memory_operation_list/
  - mod.rs
- examples/operations/send_email/
  - mod.rs
- examples/operations/send_notification/
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
  - query_kv.rs
  - query_service.rs
  - subject.rs
- src/query/
  - in_memory.rs
  - mod.rs

