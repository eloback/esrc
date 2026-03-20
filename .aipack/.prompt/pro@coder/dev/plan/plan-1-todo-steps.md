
## Step - Remove AggregateQueryHandler from esrc-cqrs
      status: not_started
time-created: 2026-03-20 20:49:02

Remove the `AggregateQueryHandler` struct and its associated types from `crates/esrc-cqrs`.

- Delete `crates/esrc-cqrs/src/nats/aggregate_query_handler.rs`.
- Remove `pub use aggregate_query_handler::{AggregateQueryHandler, QueryEnvelope, QueryReply};` from `crates/esrc-cqrs/src/nats/mod.rs`.
- Remove `mod aggregate_query_handler;` from `crates/esrc-cqrs/src/nats/mod.rs`.
- Remove the `aggregate_query_handler` comment line from `mod.rs` as well.
- Update `crates/esrc-cqrs/src/nats/query_dispatcher.rs` to remove the internal import of `QueryReply` from `aggregate_query_handler` (currently used in the error path of `NatsQueryDispatcher::run`); define a minimal `QueryReply`-like error response inline or move `QueryEnvelope`/`QueryReply` to `query_dispatcher.rs` as public types.
- Ensure `cargo check -p esrc-cqrs --features nats` passes cleanly after removal.

## Step - Introduce the View trait in esrc
      status: not_started
time-created: 2026-03-20 20:49:02

Add a `View` trait to the `esrc` crate (in `src/`) that represents a read model built from events, analogous to `Aggregate` but without commands, process, or errors.

- Create `src/view.rs` with the `View` trait:
  - Associated type `Event: event::Event` (the event stream it is built from).
  - Required method `fn apply(self, event: &Self::Event) -> Self` (same signature as `Aggregate::apply`).
  - The type must be `Default + Send`.
  - No `Command`, `process`, or `Error` associated types.
- Re-export `View` from `src/lib.rs` at the crate root.
- Add `pub mod view;` to `src/lib.rs`.
- Ensure `cargo check --features nats,derive` passes cleanly.

## Step - Introduce LiveViewQuery in esrc-cqrs
      status: not_started
time-created: 2026-03-20 20:49:02

Add `LiveViewQuery`: a `QueryHandler` for `esrc-cqrs` that replays events on each request to build a `View` and return it as the query response.

References: see the definition in plan-3-done-steps.md, step 'Step - Introduce the View trait in esrc'.

- Create `crates/esrc-cqrs/src/nats/live_view_query.rs`:
  - Define `LiveViewQuery<V, R>` where `V: View` and `R: Serialize`.
  - It accepts a handler name (`&'static str`) and a projection function `fn(&V) -> R`.
  - `QueryEnvelope` (wrapping only a `Uuid` id) and `QueryReply` should be defined (or re-exported) in `query_dispatcher.rs` as the canonical wire types for all query handlers; import them from there.
  - Implement `QueryHandler<NatsStore>` for `LiveViewQuery<V, R>`:
    1. Deserialize payload as `QueryEnvelope`.
    2. Use `ReplayOneExt::replay_one` (or `read`) to replay the view's event stream for the given ID.
    3. Fold events into a `V` instance starting from `V::default()`.
    4. Apply the projection function and serialize the result as a `QueryReply`.
- Export `LiveViewQuery` from `crates/esrc-cqrs/src/nats/mod.rs`.
- Ensure `cargo check -p esrc-cqrs --features nats` passes.

## Step - Introduce MemoryViewQuery in esrc-cqrs
      status: not_started
time-created: 2026-03-20 20:49:02

Add `MemoryViewQuery`: a `Project` implementation that keeps a `View` per aggregate ID in memory, and a `QueryHandler` that reads from that in-memory store.

References: see the definition in plan-3-done-steps.md, step 'Step - Introduce the View trait in esrc'.

- Create `crates/esrc-cqrs/src/nats/memory_view_query.rs`:
  - Define `MemoryView<V>` where `V: View + Clone`:
    - Internally holds an `Arc<RwLock<HashMap<Uuid, V>>>`.
    - Implements `Clone` (cheaply, via `Arc` clone).
    - Implements `Project` for `MemoryView<V>`:
      - `type EventGroup = V::Event`
      - `type Error = std::convert::Infallible`
      - `async fn project(...)`: deserializes the event, looks up (or inserts default) the `V` for the aggregate ID, and applies the event in-place using `V::apply`.
  - Define `MemoryViewQuery<V, R>` where `V: View + Clone`, `R: Serialize`:
    - Holds a `MemoryView<V>` (shared handle) and a projection function `fn(&V) -> R`.
    - Accepts a handler name (`&'static str`).
    - Implements `QueryHandler<NatsStore>`:
      1. Deserialize payload as `QueryEnvelope`.
      2. Lock the map read-side and look up the view for the given ID.
      3. Apply the projection function (or use `V::default()` if not present) and serialize as `QueryReply`.
  - `MemoryView<V>` should be constructable standalone so it can be registered as a projector and also referenced by one or more `MemoryViewQuery` instances.
- Export `MemoryView` and `MemoryViewQuery` from `crates/esrc-cqrs/src/nats/mod.rs`.
- Ensure `cargo check -p esrc-cqrs --features nats` passes.

## Step - Update tests and cafe example for new View-based query API
      status: not_started
time-created: 2026-03-20 20:49:02

Update the integration tests and the cafe example to use the new `LiveViewQuery` and `MemoryViewQuery` APIs instead of the removed `AggregateQueryHandler`.

References: see the definitions in plan-3-done-steps.md, steps 'Step - Introduce LiveViewQuery in esrc-cqrs' and 'Step - Introduce MemoryViewQuery in esrc-cqrs'.

- `crates/esrc-cqrs/tests/integration_nats.rs`:
  - Make `Counter` implement `View` (add `esrc::View` impl with `apply` identical to the existing `Aggregate::apply`).
  - Replace all uses of `AggregateQueryHandler::<Counter, CounterState>` with `LiveViewQuery::<Counter, CounterState>` (or `MemoryViewQuery` where appropriate for the test intent).
  - Update imports accordingly.
  - Ensure all existing query tests still pass semantically (same assertions, same wire format via `QueryReply`).

- `examples/cafe/`:
  - Make `Order` (in `examples/cafe/domain.rs`) implement `View`.
  - In `examples/cafe/main.rs`, replace `AggregateQueryHandler::<Order, OrderState>` with `LiveViewQuery::<Order, OrderState>` (keeping `OrderState::from_root` as the projection, adapted to take `&Order` instead of `&Root<Order>` if needed, or keep the function signature compatible).
  - Update imports in `main.rs`.
  - Ensure `cargo check --example cafe --features nats,derive` passes cleanly.
