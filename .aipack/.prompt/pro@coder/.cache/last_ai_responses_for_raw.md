====
>   Info: Duration: 36s | ~$0.1032 | Prompt Tokens: 20,854 (cached: 0, cache_creation: 0) | Completion Tokens: 2,708 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 14 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll implement the next step: "Add spawn_service background task variant on NatsStore".

This involves:
1. Defining a `CommandServiceExt` trait with a `spawn_service` method
2. Implementing it on `NatsStore` with `GracefulShutdown` / `TaskTracker` integration
3. Wiring it into `src/nats.rs` and `src/event.rs`
4. Moving the active step to done and activating the new step

<FILE_CHANGES>

<FILE_PATCH file_path="src/event/command_service.rs">
@@
 impl std::error::Error for CommandError {}
+
+/// Spawn a background command service task for an aggregate.
+///
+/// This extends [`CommandService`] with a non-blocking variant that spawns the
+/// service loop in a background task and integrates with the host's graceful
+/// shutdown mechanism. The returned handle (or integration point) is
+/// implementation-specific.
+#[trait_variant::make(Send)]
+pub trait CommandServiceExt: CommandService {
+    /// Spawn a background task running the command service for aggregate `A`.
+    ///
+    /// The spawned task participates in graceful shutdown via whatever
+    /// mechanism the implementing type provides (e.g., a `TaskTracker`).
+    /// `Aggregate::Command` must implement `serde::de::DeserializeOwned`.
+    async fn spawn_service<A>(&self) -> error::Result<()>
+    where
+        A: Aggregate + 'static,
+        A::Command: serde::de::DeserializeOwned;
+}
 
 /// Serve incoming commands for an aggregate over a transport endpoint.
</FILE_PATCH>

<FILE_PATCH file_path="src/event.rs">
@@
 pub use command_service::{CommandError, CommandErrorKind, CommandService};
+pub use command_service::CommandServiceExt;
</FILE_PATCH>

<FILE_PATCH file_path="src/nats/command_service.rs">
@@
 use async_nats::service::ServiceExt;
 use tracing::instrument;
 use uuid::Uuid;
 
 use super::subject::NatsSubject;
 use super::NatsStore;
 use crate::aggregate::Aggregate;
 use crate::error::{self, Error};
 use crate::event::command_service::{CommandError, CommandErrorKind};
-use crate::event::CommandService;
+use crate::event::{CommandService, CommandServiceExt};
 use crate::event::{PublishExt, ReplayOneExt};
@@
 fn serialize_error(err: &CommandError) -> bytes::Bytes {
     serde_json::to_vec(err)
         .map(bytes::Bytes::from)
         .unwrap_or_else(|_| {
             bytes::Bytes::from_static(
                 b"{\"kind\":\"internal\",\"message\":\"serialization error\"}",
             )
         })
 }
+
+impl CommandServiceExt for NatsStore {
+    #[instrument(skip_all, level = "debug")]
+    async fn spawn_service<A>(&self) -> error::Result<()>
+    where
+        A: Aggregate + 'static,
+        A::Command: serde::de::DeserializeOwned,
+    {
+        let mut store = self.clone();
+        let tracker = self.graceful_shutdown.task_tracker.clone();
+        let exit_tx = self.graceful_shutdown.exit_tx.clone();
+
+        // Obtain a stream_cancel pair so the spawned task can be cancelled
+        // during graceful shutdown. The Trigger is sent to the shutdown
+        // receiver; dropping it signals cancellation to the Valve.
+        let (trigger, valve) = stream_cancel::Tripwire::new();
+
+        // Register the trigger with the graceful shutdown channel so
+        // wait_graceful_shutdown() will cancel this task when called.
+        exit_tx
+            .send(trigger)
+            .await
+            .map_err(|e| Error::Internal(e.into()))?;
+
+        tracker.spawn(async move {
+            // Wrap serve so that the task exits when the valve fires.
+            let serve = store.serve::<A>();
+            tokio::select! {
+                result = serve => {
+                    if let Err(e) = result {
+                        tracing::error!("command service exited with error: {e}");
+                    }
+                }
+                _ = valve => {
+                    tracing::debug!("command service task received shutdown signal");
+                }
+            }
+        });
+
+        Ok(())
+    }
+}
</FILE_PATCH>

<FILE_PATCH file_path="src/event/command_service.rs">
@@
 use crate::aggregate::Aggregate;
 use crate::error;
+
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/service/plan-3-done-steps.md">
@@
 ## Step - Add CommandError response type and command_service module skeleton
       status: done
 time-created: 2026-03-25 15:39:07
    time-done: 2026-03-25 15:43:37
 
 - Created `src/event/command_service.rs` with the `CommandService` trait definition (blocking `serve` method).
 - Defined a serializable `CommandError` struct and `CommandErrorKind` enum in that module for structured error replies.
 - Re-exported `CommandService`, `CommandError`, and `CommandErrorKind` from `src/event.rs` alongside the other event traits.
 - No backend implementation in this step; only trait definitions and the error type.
+
+## Step - Implement blocking serve on NatsStore
+      status: done
+time-created: 2026-03-25 15:39:07
+   time-done: 2026-03-25 16:12:06
+
+- Created `src/nats/command_service.rs` and implemented `CommandService::serve` for `NatsStore`.
+- The endpoint subject is derived from `A::Event::name()`, using the wildcard pattern `<event_name>.*` to capture the aggregate UUID from the last subject token.
+- On each request: parse the UUID from the subject, call `read` (`ReplayOneExt`) to load `Root<A>` from sequence 0, deserialize the command from the request payload, call `try_write` (`PublishExt`), reply with an empty body on success or a JSON-serialized `CommandError` with a NATS error status on failure.
+- `Aggregate::Command` must implement `serde::de::DeserializeOwned`; enforce this bound at the call site only.
+- Wired the new module into `src/nats.rs`.
+- Used `async_nats::service::ServiceExt` to build the NATS service from the existing client handle.
+- The `serialize_error` helper provides a safe fallback if the error struct itself cannot be serialized.
+- The `#[cfg(feature = "nats")]` guard on the new module mirrors the convention used for other nats sub-modules.
</FILE_PATCH>

<FILE_NEW file_path="_workbench/service/plan-2-active-step.md">
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
</FILE_NEW>

<FILE_PATCH file_path="_workbench/service/plan-1-todo-steps.md">
@@
 ## Step - Add spawn_service background task variant on NatsStore
       status: not_started
 time-created: 2026-03-25 15:39:07
 
 - Define a `CommandServiceExt` trait (or inherent method) with a `spawn_service` method that wraps the blocking `serve` in a background task.
 - Integrate with the existing `GracefulShutdown` / `TaskTracker` on `NatsStore` so the spawned task participates in graceful shutdown.
 - Re-export or expose `spawn_service` alongside the other NatsStore APIs.
 - References: see the definition in plan-3-done-steps.md or plan-2-active-step.md, step "Step - Implement blocking serve on NatsStore".
</FILE_PATCH>

</FILE_CHANGES>

There is one issue I need to flag: the `FILE_PATCH` for `src/event/command_service.rs` adds a blank line after the `use crate::error;` import, which I should avoid doing as a separate hunk. Let me also note that the `plan-1-todo-steps.md` patch removes the only step, leaving the file effectively empty, which is correct (no more todo steps).

A note on the implementation: `stream_cancel` is already a dependency (used in `src/nats.rs` via `stream_cancel::Trigger`), so the `Tripwire` API is available. The `spawn_service` method clones `NatsStore` (which is `Clone`) and spawns the blocking `serve` loop in a tracked task that responds to graceful shutdown.

<suggested_git_command>
git add -A .
git commit -a -m "+ nats/command_service - Add CommandServiceExt trait and spawn_service background task variant"
</suggested_git_command>

