## Step - Introduce the View trait in esrc
      status: active
time-created: 2026-03-20 20:49:02
time-current: 2026-03-20 22:43:57

Add a `View` trait to the `esrc` crate (in `src/`) that represents a read model built from events, analogous to `Aggregate` but without commands, process, or errors.

- Create `src/view.rs` with the `View` trait:
  - Associated type `Event: event::Event` (the event stream it is built from).
  - Required method `fn apply(self, event: &Self::Event) -> Self` (same signature as `Aggregate::apply`).
  - The type must be `Default + Send`.
  - No `Command`, `process`, or `Error` associated types.
- Re-export `View` from `src/lib.rs` at the crate root.
- Add `pub mod view;` to `src/lib.rs`.
- Ensure `cargo check --features nats,derive` passes cleanly.
