## Step - Add spawn_service background task variant on NatsStore
      status: active
time-created: 2026-03-25 15:39:07
time-current: 2026-03-25 17:56:52

- Defined `spawn_service` as an inherent method on `NatsStore` (not a trait, since it is NATS-specific).
- Uses `Tripwire` from `stream_cancel` for cancellation, matching the existing pattern in the codebase.
- The spawned task is registered with the `TaskTracker` and the `Trigger` is sent to the `exit_tx` channel so that `wait_graceful_shutdown` cancels it.
- `tokio::select!` is used to race between `serve` completing and the tripwire being triggered.
- References: see the definition in plan-3-done-steps.md, step "Step - Implement blocking serve on NatsStore".
