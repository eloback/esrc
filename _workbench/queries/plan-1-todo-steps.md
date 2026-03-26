# Plan 1 - Todo Steps

## Step - Implement NATS QueryService and QueryClient
      status: not_started
time-created: 2026-03-26 14:58:37

- Create `src/nats/query_service.rs` (or extend existing nats module structure).

- Implement `QueryService` for `NatsStore`:
  - Derive NATS request-reply subject from `ComponentName` segments: `query.<bounded_context>.<domain>.<feature>.<component>`.
  - Deserialize incoming query requests, dispatch to the `QueryHandler`, serialize and send back results.
  - Follow the same pattern as `CommandService` in `src/nats/command_service.rs` (service builder, endpoint, request/reply loop).
  - Include serializable reply/error types analogous to `CommandReply`/`ReplyError`.

- Implement `QueryClient` for `NatsStore`:
  - Serialize query, send as NATS request to the derived subject, deserialize reply.
  - Map transport/service errors back to `esrc::error::Error`.

- Add `spawn_query_service` method to `NatsStore` for spawning with graceful shutdown (following `spawn_service` pattern).

- Register the new module in `src/nats.rs`.

- References: see the `QueryService`/`QueryClient` traits defined in `src/query.rs` (previous step), and the `CommandService`/`CommandClient` NATS implementation in `src/nats/command_service.rs` for the pattern to follow.
