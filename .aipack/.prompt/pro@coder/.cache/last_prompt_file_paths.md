
====
file_content_mode: udiffx

# Knowledge Files

## All resolve files(sent to AI with content, in this order)

- docs/skill/esrc-slice-constants-and-module-layout.md
- docs/skill/esrc-event-modeling-create-consumers-automations-and-read-models.md
- docs/skill/esrc-command-service-execute-commands.md
- docs/skill/esrc-read-model-public-interface-and-queries.md

# Context Files

## All resolve files(sent to AI with content, in this order)

- src/event.rs
- src/event/command_service.rs
- examples/multi-slice-command-service/domain.rs
- src/nats/command_service.rs
- examples/multi-slice-command-service/main.rs
- examples/multi-slice-command-service/queue_welcome_email/mod.rs
- src/nats/query_service.rs
- examples/basic-query-service/main.rs
- examples/basic-query-service/read_model.rs
- src/aggregate.rs
- src/query/mod.rs
- src/query/in_memory.rs
- src/nats/query_kv.rs
- src/event_modeling.rs
- src/nats.rs

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
  - query_kv.rs
  - query_service.rs
  - subject.rs
- src/query/
  - in_memory.rs
  - mod.rs

