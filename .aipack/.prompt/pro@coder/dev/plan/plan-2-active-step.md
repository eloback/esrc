## Step - Update tests and cafe example for new View-based query API
      status: active
time-created: 2026-03-20 20:49:02
time-current: 2026-03-21 03:09:04

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

### Implementation

- Added `use esrc::view::View;` import to `integration_nats.rs`.
- Added `View` impl for `Counter` mirroring `Aggregate::apply`.
- Added `LiveViewQuery` and `MemoryView`/`MemoryViewQuery` to imports in `integration_nats.rs`.
- Replaced all four inline `CounterGetState`/`CounterGetStateAlt` struct-based query handlers in `integration_nats.rs` with `LiveViewQuery::<Counter, CounterState>::new(...)` calls using `|v| CounterState { value: v.value }` as the projection.
- In `examples/cafe/domain.rs`: added `use esrc::view::View;`, renamed `OrderState::from_root` to `OrderState::from_order` (takes `&Order` directly instead of `&Root<Order>`), and added `View` impl for `Order` mirroring `Aggregate::apply`.
- In `examples/cafe/main.rs`: rewrote the file cleanly, removing the duplicate import blocks for `AggregateCommandHandler` etc., replacing `AggregateQueryHandler::<Order, OrderState>::new("Order.GetState", OrderState::from_root)` with `LiveViewQuery::<Order, OrderState>::new("Order.GetState", OrderState::from_order)`, and removing the `ConnectOptions` unused import.

### Bug fix - LiveViewQuery used non-existent read_one API
time-current: 2026-03-21 03:09:04

#### User ask

Execute next step (triggered review of active step before finalizing).

#### AI Answer

`LiveViewQuery::handle` was calling `store.read_one::<V::Event>(envelope.id)` which does not exist on `NatsStore`. The correct API is `ReplayOne::replay_one` which returns a stream of `NatsEnvelope` items. Fixed by:
- Replacing the `read_one` call with `store.replay_one::<V::Event>(envelope.id, Sequence::new())`.
- Deserializing each envelope via `nats_envelope.deserialize::<V::Event>()` before applying it to the view.
- Updating imports in `live_view_query.rs`: added `Error`, `ReplayOne`, `StreamExt`, `Uuid`; removed the inner `use futures::StreamExt` block.
