# Plan 2 - Active Step

## Step - Vertical slice helper composing ConsumerSpec and QuerySpec
      status: active
time-created: 2026-03-26 15:39:52
time-current: 2026-03-26 16:54:53

- Create a convenience builder or type that declares a `ReadModel` consumer (`ConsumerSpec`) and its `QueryHandler` (`QuerySpec`) together as a single vertical slice.
- This simplifies the developer experience for the common case where a read model has both an event consumer (write side) and a query handler (read side).
- Design considerations:
  - Should compose, not replace, the existing `ConsumerSpec` and `QuerySpec` types.
  - A builder pattern (e.g., `ReadModelSlice::new(name, projector, handler)`) that produces both specs, or a struct that holds both and can be passed to `NatsStore` for spawning.
  - Consider adding a `spawn_read_model_slice` or similar convenience method on `NatsStore` that spawns both the consumer and the query service in one call.
  - Be mindful of how these components are registered in the runtime (the user emphasized developer UX and runtime registration flow).
- Module placement: likely in `src/event_modeling.rs` since it is a declaration/composition concern, or a new `src/slice.rs` if it grows.
- References: see `src/event_modeling.rs` for `ConsumerSpec`, `ReadModel`, `Automation`; see `src/query.rs` for `QuerySpec`.

