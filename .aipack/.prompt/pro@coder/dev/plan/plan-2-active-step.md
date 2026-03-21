## Step - Introduce LiveViewQuery in esrc-cqrs
      status: active
time-created: 2026-03-20 20:49:02
time-current: 2026-03-20 22:57:58

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

### Implementation Considerations

The `View` trait's `apply` method consumes events by value, while `Aggregate::apply` does the same. The replay loop uses `store.read_one::<V::Event>(envelope.id)` which produces deserialized events directly (not envelopes), folding them into `V::default()` via `View::apply`.

The `ReplayOneExt::read` method (from `src/event/replay.rs`) returns a stream of deserialized events for a single aggregate ID, which is the appropriate API to use here. The `read_one` method is the deserialized-event variant of `replay_one`.
