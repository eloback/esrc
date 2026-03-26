# Plan 2 - Active Step

## Step - Implement NATS QueryService and QueryClient
      status: active
time-created: 2026-03-26 14:58:37
time-current: 2026-03-26 15:18:42

- Created `src/nats/query_service.rs` with full NATS-backed implementations.

- Implemented `QueryService` for `NatsStore`:
  - Derives NATS request-reply subject from `ComponentName::query_subject()`: `query.<bounded_context>.<domain>.<feature>.<component>`.
  - Uses the async-nats service builder to create a service group and endpoint, following the same pattern as `CommandService` in `src/nats/command_service.rs`.
  - Deserializes incoming `QueryRequest` (either `GetById` or `Query` variant), dispatches to the `QueryHandler`, serializes and sends back results.
  - Includes serializable reply/error types: `QueryRequest`, `QueryReplyError`, `GetByIdReply`, `QueryReply`.
  - Error handling follows the same pattern as `CommandReply`/`ReplyError`, with fallback error replies if serialization fails.

- Implemented `QueryClient` for `NatsStore`:
  - `get_by_id`: serializes a `QueryRequest::GetById`, sends as NATS request to `<query_subject>.query`, deserializes `GetByIdReply`.
  - `query`: serializes a `QueryRequest::Query`, sends as NATS request, deserializes `QueryReply`.
  - Maps transport/service errors back to `esrc::error::Error::Internal`.

- Added `spawn_query_service` method to `NatsStore`:
  - Follows the `spawn_service` pattern with `Tripwire`-based graceful shutdown.
  - Registers a shutdown trigger, wraps `serve` in a tracked cancellable task.

- Registered `pub mod query_service` in `src/nats.rs`.
