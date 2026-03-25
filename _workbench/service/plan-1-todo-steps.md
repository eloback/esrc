## Step - Add spawn_service background task variant on NatsStore
      status: not_started
time-created: 2026-03-25 15:39:07

- Define a `CommandServiceExt` trait (or inherent method) with a `spawn_service` method that wraps the blocking `serve` in a background task.
- Integrate with the existing `GracefulShutdown` / `TaskTracker` on `NatsStore` so the spawned task participates in graceful shutdown.
- Re-export or expose `spawn_service` alongside the other NatsStore APIs.
- References: see the definition in plan-3-done-steps.md or plan-2-active-step.md, step "Step - Implement blocking serve on NatsStore".
