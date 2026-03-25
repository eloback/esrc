## Step - Add spawn_service background task variant on NatsStore
      status: active
time-created: 2026-03-25 15:39:07
time-current: 2026-03-25 16:12:06

- Define a `CommandServiceExt` trait (or inherent method) with a `spawn_service` method that wraps the blocking `serve` in a background task.
- Integrate with the existing `GracefulShutdown` / `TaskTracker` on `NatsStore` so the spawned task participates in graceful shutdown.
- Re-export or expose `spawn_service` alongside the other NatsStore APIs.
- References: see the definition in plan-3-done-steps.md, step "Step - Implement blocking serve on NatsStore".

### Implementation Considerations

- `CommandServiceExt` trait is defined in `src/event/command_service.rs` alongside `CommandService`, and re-exported from `src/event.rs`.
- The `spawn_service` implementation on `NatsStore` clones the store (which is `Clone`) and spawns a `tokio` task via the existing `TaskTracker`.
- `stream_cancel::Tripwire` is used to produce a `(Trigger, Valve)` pair; the `Trigger` is registered with the `GracefulShutdown` exit channel so `wait_graceful_shutdown()` will cancel the task by dropping the trigger, which fires the valve.
- The spawned task uses `tokio::select!` to race `serve` against the valve, ensuring clean shutdown on signal.
- `A: 'static` bound is required because the spawned task must be `'static`.
