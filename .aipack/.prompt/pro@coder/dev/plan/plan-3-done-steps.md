## Step - Remove AggregateQueryHandler from esrc-cqrs
      status: done
time-created: 2026-03-20 20:49:02
   time-done: 2026-03-20 22:36:06

- Deleted `crates/esrc-cqrs/src/nats/aggregate_query_handler.rs`.
- Moved `QueryEnvelope` and `QueryReply` into `crates/esrc-cqrs/src/nats/query_dispatcher.rs` as public types; these are the canonical wire types for all query handlers.
- Removed `pub use aggregate_query_handler::{AggregateQueryHandler, QueryEnvelope, QueryReply};` from `mod.rs` and replaced with `pub use query_dispatcher::{NatsQueryDispatcher, QueryEnvelope, QueryReply};`.
- Removed `mod aggregate_query_handler;` and its associated comment from `mod.rs`.
- Removed the internal `use crate::nats::aggregate_query_handler::QueryReply;` from the error path in `query_dispatcher.rs` (now uses the locally defined `QueryReply`).
- Updated `crates/esrc-cqrs/tests/integration_nats.rs`: replaced all `AggregateQueryHandler` usages with a `counter_query_handler` helper function that implements `QueryHandler<NatsStore>` directly; updated imports.
- Updated `examples/cafe/main.rs`: replaced `AggregateQueryHandler` with an `order_state_query_handler` helper function; removed the unused `AggregateQueryHandler` and `ConnectOptions` imports; kept `OrderState::from_root` as the projection.

## Step - Introduce the View trait in esrc
      status: done
time-created: 2026-03-20 20:49:02
   time-done: 2026-03-20 22:57:58

Add a `View` trait to the `esrc` crate (in `src/`) that represents a read model built from events, analogous to `Aggregate` but without commands, process, or errors.

- `src/view.rs` defines the `View` trait with:
  - Associated type `Event: event::Event`.
  - Required method `fn apply(self, event: &Self::Event) -> Self`.
  - Bound `Default + Send` on the implementing type.
  - No `Command`, `process`, or `Error` associated types.
- `src/lib.rs` exports `View` from the crate root via `pub mod view;` and `pub use view::View;`.

## Step - Introduce LiveViewQuery in esrc-cqrs
      status: done
time-created: 2026-03-20 20:49:02
   time-done: 2026-03-20 23:02:05

Add `LiveViewQuery`: a `QueryHandler` for `esrc-cqrs` that replays events on each request to build a `View` and return it as the query response.

- Created `crates/esrc-cqrs/src/nats/live_view_query.rs`:
  - Defines `LiveViewQuery<V, R>` where `V: View` and `R: Serialize`.
  - Accepts a handler name (`&'static str`) and a projection function `fn(&V) -> R`.
  - Implements `QueryHandler<NatsStore>`: deserializes payload as `QueryEnvelope`, replays events via `store.read_one::<V::Event>(envelope.id)`, folds into `V::default()`, applies projection, returns `QueryReply`.
- Exported `LiveViewQuery` from `crates/esrc-cqrs/src/nats/mod.rs`.

## Step - Introduce MemoryViewQuery in esrc-cqrs
      status: done
time-created: 2026-03-20 20:49:02
   time-done: 2026-03-20 23:02:05

Add `MemoryView` and `MemoryViewQuery` to `esrc-cqrs`.

- Created `crates/esrc-cqrs/src/nats/memory_view_query.rs`:
  - `MemoryView<V>` holds `Arc<RwLock<HashMap<Uuid, V>>>`, implements `Clone`, `Default`, and `Project` (with `type EventGroup = V::Event`, `type Error = Infallible`). The `project` method looks up or inserts a default `V` for the aggregate ID and applies the event in-place.
  - `MemoryViewQuery<V, R>` holds a `MemoryView<V>` handle, a handler name, and a projection function. Implements `QueryHandler<NatsStore>`: deserializes payload as `QueryEnvelope`, takes a read-lock snapshot of the view for the requested ID (defaulting to `V::default()` if absent), applies the projection, returns `QueryReply`.
- Exported `MemoryView` and `MemoryViewQuery` from `crates/esrc-cqrs/src/nats/mod.rs`.
- Added `mod memory_view_query;` with doc comment to `mod.rs`.
