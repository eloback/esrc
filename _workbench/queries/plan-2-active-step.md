# Plan 2 - Active Step

## Step - Add QueryService and QueryClient traits to src/query.rs
      status: active
time-created: 2026-03-26 14:58:37
time-current: 2026-03-26 15:15:09

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
