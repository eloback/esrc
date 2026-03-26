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
