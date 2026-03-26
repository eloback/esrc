
====
file_content_mode: udiffx

# Context Files

## All resolve files(sent to AI with content, in this order)

- src/event/command_service.rs
- docs/skill/esrc-command-service-execute-commands.md
- docs/skill/esrc-read-model-public-interface-and-queries.md
- src/query/mod.rs

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
  - lib.rs
  - nats.rs
  - project.rs
  - version.rs
- src/event/
  - command_service.rs
  - future.rs
  - publish.rs
  - replay.rs
  - subscribe.rs
  - truncate.rs
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

