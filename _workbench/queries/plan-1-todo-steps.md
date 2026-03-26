# Plan 1 - Todo Steps

## Step - Rename ConsumerName to ComponentName in event_modeling.rs
      status: not_started
time-created: 2026-03-26 14:58:37

- Rename the `ConsumerName` struct to `ComponentName` in `src/event_modeling.rs`.
- Rename the `consumer` field to `component` (and update the associated `consumer()` method to `component()`).
- Update `durable_name()` and `slice_path()` to use the new field name.
- Update all references in `ConsumerSpec`, `Automation`, `ReadModel` builders within the same file.
- Keep `ConsumerRole`, `ConsumerSpec`, `Automation`, and `ReadModel` names unchanged.
- Update all usages across the codebase: `src/nats.rs`, `src/nats/command_service.rs`, example files, and any other files that reference `ConsumerName` or the `consumer` field/method.

## Step - Create src/query.rs with Query, QueryHandler, QueryTransport, and QuerySpec
      status: not_started
time-created: 2026-03-26 14:58:37

- Create `src/query.rs` as a single file containing:

- `Query` trait:
  - `type ReadModel: Send`
  - `type Response: Send`
  - No serde bounds.

- `QueryHandler` trait (using `#[trait_variant::make(Send)]`):
  - `type Query: Query`
  - `type Id: Send + Sync`
  - `async fn get_by_id(&self, id: Self::Id) -> crate::error::Result<Option<<Self::Query as Query>::ReadModel>>`
  - `async fn handle(&self, query: Self::Query) -> crate::error::Result<<Self::Query as Query>::Response>`
  - Requires `Send + Sync`, no `Clone`.

- `QueryTransport` enum:
  - `NatsRequestReply` variant (only one for now, extensible).

- `QuerySpec` struct:
  - Holds a `ComponentName` (from `event_modeling`), a `QueryTransport`, and the `QueryHandler` instance.
  - Constructor and accessor methods following the `ConsumerSpec` pattern.

- Register the module in `src/lib.rs` with a doc comment.

## Step - Add QueryService and QueryClient traits to src/query.rs
      status: not_started
time-created: 2026-03-26 14:58:37

- Add transport-agnostic trait definitions to `src/query.rs`, following the `CommandService`/`CommandClient` pattern from `src/event/command_service.rs`.

- `QueryService` trait:
  - Method to serve/handle incoming query requests.
  - Serde bounds (`Serialize + DeserializeOwned`) added at the method level on the query enum and response types.

- `QueryClient` trait:
  - Method to send a query remotely and await the response.
  - Serde bounds at the method level.

- Both traits use `#[trait_variant::make(Send)]`.
- Subject derivation convention documented: `query.<bounded_context>.<domain>.<feature>.<component>`.

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
