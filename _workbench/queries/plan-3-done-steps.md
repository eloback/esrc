# Plan 3 - Done Steps

## Step - Rename ConsumerName to ComponentName in event_modeling.rs
      status: done
time-created: 2026-03-26 14:58:37
   time-done: 2026-03-26 15:04:08

- Renamed the `ConsumerName` struct to `ComponentName` in `src/event_modeling.rs`.
- Renamed the `consumer` field to `component` (and updated the associated `consumer()` method to `component()`).
- Updated `durable_name()` and `slice_path()` to use the new field name.
- Updated all references in `ConsumerSpec`, `Automation`, `ReadModel` builders within the same file.
- Kept `ConsumerRole`, `ConsumerSpec`, `Automation`, and `ReadModel` names unchanged.
- Updated all usages across the codebase: example files that reference `ConsumerName` or the `consumer` field/method.
- `src/nats.rs` and `src/nats/command_service.rs` do not directly reference `ConsumerName` by import (they use `ConsumerSpec` which internally holds `ComponentName`), so no changes needed there.

## Step - Create src/query.rs with Query, QueryHandler, QueryTransport, and QuerySpec
      status: done
time-created: 2026-03-26 14:58:37
   time-done: 2026-03-26 15:15:09

- Created `src/query.rs` as a single file containing:

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

- Registered the module in `src/lib.rs` with a doc comment.

## Step - Add QueryService and QueryClient traits to src/query.rs
      status: done
time-created: 2026-03-26 14:58:37
   time-done: 2026-03-26 15:18:42

- Added transport-agnostic trait definitions to `src/query.rs`, following the `CommandService`/`CommandClient` pattern from `src/event/command_service.rs`.

- `QueryService` trait (using `#[trait_variant::make(Send)]`):
  - `async fn serve<H>(&self, spec: &QuerySpec<H>) -> error::Result<()>` method to serve/handle incoming query requests.
  - Serde bounds (`Serialize + DeserializeOwned`) added at the method level on the query enum, response, and read model types.
  - The `QueryHandler::Id` also requires `DeserializeOwned` for receiving `get_by_id` requests.

- `QueryClient` trait (using `#[trait_variant::make(Send)]`):
  - `async fn get_by_id<Q, Id>(&self, name: &ComponentName, id: Id) -> error::Result<Option<<Q as Query>::ReadModel>>` for the built-in by-ID query.
  - `async fn query<Q>(&self, name: &ComponentName, query: Q) -> error::Result<Q::Response>` for custom queries.
  - Serde bounds at the method level.

- Both traits use `#[trait_variant::make(Send)]`.
- Subject derivation convention: `query.<bounded_context>.<domain>.<feature>.<component>` (documented via `ComponentName::query_subject()` helper method on `ComponentName`).

## Step - In-memory QueryHandler helper for View-based live projections
      status: done
time-created: 2026-03-26 15:39:52
   time-done: 2026-03-26 16:28:37

- Created `src/query/in_memory.rs` with `InMemoryViewStore<RM, Q>`, a thread-safe, in-memory store for read model instances keyed by `Uuid`.

- `InMemoryViewStore` provides:
  - `new(query_fn)`: constructor with closure for custom query logic.
  - `upsert`, `remove`, `get`, `all`, `len`, `is_empty` methods.

- Implements `QueryHandler` with `type Id = Uuid`, delegating to the store and closure.

- Converted `src/query.rs` to `src/query/mod.rs` module directory.

- Store is cheaply cloneable (`Arc`-wrapped), shareable between `Project` (write) and `QuerySpec` (read).

## Step - Implement NATS QueryService and QueryClient
      status: done
time-created: 2026-03-26 14:58:37
   time-done: 2026-03-26 15:36:55

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

## Step - NATS KV-backed QueryHandler implementation
      status: done
time-created: 2026-03-26 15:39:52
   time-done: 2026-03-26 16:54:53

- Created `src/nats/query_kv.rs` with `NatsKvStore<RM, Q>`, a shared read/write store backed by NATS JetStream Key-Value.

- `NatsKvStore` provides:
  - `new(nats_store, name, query_fn)`: constructor deriving bucket name from `ComponentName::durable_name()` prefixed with `rm_`.
  - `with_bucket_name`, `from_context`: alternative constructors for custom bucket names or direct JetStream context usage.
  - `put`, `delete`, `get`: CRUD operations serializing read models as JSON.
  - `bucket()`: access to the underlying NATS KV bucket for advanced operations.

- Implements `QueryHandler` with `type Id = String`, delegating `get_by_id` to `get` and `handle` to the user-supplied closure.

- Store is cheaply cloneable (`Arc`-wrapped), shareable between `Project` (write) and `QuerySpec` (read).

- Registered `pub mod query_kv` in `src/nats.rs`.
