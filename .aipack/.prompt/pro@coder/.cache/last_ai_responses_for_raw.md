====
>   Info: Duration: 27s | ~$0.159 | Prompt Tokens: 21,632 (cached: 0, cache_creation: 0) | Completion Tokens: 2,035 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 14 | Working Files: 0 | Knowledge Files: 2
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.



Looking at the plan, the current active step "Implement blocking serve on NatsStore" is already implemented. The next step in `plan-1-todo-steps.md` is "Add spawn_service background task variant on NatsStore".

Let me implement this step: move the active step to done, activate the next step, and implement it.

I need to add a `spawn_service` method that wraps the blocking `serve` in a background task, integrated with the existing `GracefulShutdown` / `TaskTracker` on `NatsStore`.

Looking at how `GracefulShutdown` works in `src/nats.rs`, I can see:
- `task_tracker: TaskTracker` for tracking spawned tasks
- `exit_tx: Sender<Trigger>` / `exit_rx` for shutdown triggers using `stream_cancel`
- `wait_graceful_shutdown` drops triggers to cancel and then waits

I'll add `spawn_service` as an inherent method on `NatsStore` (not a trait, since it's NATS-specific) that spawns `serve` in a tracked task with cancellation support.

<FILE_CHANGES>

<FILE_PATCH file_path="src/nats/command_service.rs">
@@
 use async_nats::service::ServiceExt;
 use futures::StreamExt;
+use stream_cancel::Tripwire;
 use tracing::instrument;
 
 use crate::aggregate::Aggregate;
@@
 use crate::event::command_service::{CommandError, CommandErrorKind, CommandService};
 use crate::event::publish::PublishExt;
 use crate::event::replay::ReplayOneExt;
-use crate::event::{Event, Sequence};
+use crate::event::Event;
 use crate::version::{DeserializeVersion, SerializeVersion};
 
 use super::NatsStore;
@@
     }
 }
 
+impl NatsStore {
+    /// Spawn `serve` as a background task integrated with graceful shutdown.
+    ///
+    /// This wraps [`CommandService::serve`] in a tracked, cancellable task
+    /// using the `GracefulShutdown` / `TaskTracker` already present on
+    /// `NatsStore`. The task will be cancelled when
+    /// [`NatsStore::wait_graceful_shutdown`] is called.
+    pub fn spawn_service<A>(&self)
+    where
+        A: Aggregate + 'static,
+        A::Event: SerializeVersion + DeserializeVersion,
+        A::Command: serde::de::DeserializeOwned + Send,
+    {
+        let store = self.clone();
+        let (trigger, tripwire) = Tripwire::new();
+
+        let exit_tx = self.graceful_shutdown.exit_tx.clone();
+        let task_tracker = self.graceful_shutdown.task_tracker.clone();
+
+        task_tracker.spawn(async move {
+            // Register the trigger so it is cancelled during graceful shutdown.
+            if exit_tx.send(trigger).await.is_err() {
+                tracing::warn!("failed to register shutdown trigger for command service");
+                return;
+            }
+
+            tokio::select! {
+                result = store.serve::<A>() => {
+                    if let Err(e) = result {
+                        tracing::error!("command service for '{}' exited with error: {e}", A::Event::name());
+                    }
+                }
+                _ = tripwire => {
+                    tracing::info!("command service for '{}' shutting down gracefully", A::Event::name());
+                }
+            }
+        });
+    }
+}
+
 async fn reply_error(request: &async_nats::service::Request, error: CommandError) {
     let payload = serde_json::to_vec(&error).unwrap_or_default();
     if let Err(e) = request
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/service/plan-1-todo-steps.md">
@@
-## Step - Add spawn_service background task variant on NatsStore
-      status: not_started
-time-created: 2026-03-25 15:39:07
-
-- Define a `CommandServiceExt` trait (or inherent method) with a `spawn_service` method that wraps the blocking `serve` in a background task.
-- Integrate with the existing `GracefulShutdown` / `TaskTracker` on `NatsStore` so the spawned task participates in graceful shutdown.
-- Re-export or expose `spawn_service` alongside the other NatsStore APIs.
-- References: see the definition in plan-3-done-steps.md or plan-2-active-step.md, step "Step - Implement blocking serve on NatsStore".
</FILE_PATCH>

<FILE_NEW file_path="_workbench/service/plan-2-active-step.md">
## Step - Add spawn_service background task variant on NatsStore
      status: active
time-created: 2026-03-25 15:39:07
time-current: 2026-03-25 17:56:52

- Defined `spawn_service` as an inherent method on `NatsStore` (not a trait, since it is NATS-specific).
- Uses `Tripwire` from `stream_cancel` for cancellation, matching the existing pattern in the codebase.
- The spawned task is registered with the `TaskTracker` and the `Trigger` is sent to the `exit_tx` channel so that `wait_graceful_shutdown` cancels it.
- `tokio::select!` is used to race between `serve` completing and the tripwire being triggered.
- References: see the definition in plan-3-done-steps.md, step "Step - Implement blocking serve on NatsStore".
</FILE_NEW>

<FILE_PATCH file_path="_workbench/service/plan-3-done-steps.md">
@@
 ## Step - Add CommandError response type and command_service module skeleton
       status: done
 time-created: 2026-03-25 15:39:07
    time-done: 2026-03-25 17:54:46
 
 - Created `src/event/command_service.rs` with the `CommandService` trait definition (blocking `serve` method).
 - Defined a serializable `CommandError` struct and `CommandErrorKind` enum for structured error replies.
 - Re-exported `CommandService`, `CommandError`, and `CommandErrorKind` from `src/event.rs` alongside the other event traits.
+
+## Step - Implement blocking serve on NatsStore
+      status: done
+time-created: 2026-03-25 15:39:07
+   time-done: 2026-03-25 17:56:52
+
+- Created `src/nats/command_service.rs` and implemented `CommandService::serve` for `NatsStore`.
+- The endpoint subject is derived from `A::Event::name()`, using the wildcard pattern `<event_name>.*` to capture the aggregate UUID from the last subject token.
+- On each request: parse the UUID from the subject, call `read` (`ReplayOneExt`) to load `Root<A>` from sequence 0, deserialize the command from the request payload, call `try_write` (`PublishExt`), reply with an empty body on success or a JSON-serialized `CommandError` with a NATS error status on failure.
+- Added `status_code()` method to `CommandError` for mapping error kinds to HTTP-like status codes used by the NATS service error API.
+- The `serve` method clones `NatsStore` for each request to obtain a mutable handle for `try_write`, since `NatsStore` is `Clone`.
+- Service and group names are derived from `A::Event::name()`, endpoint is named `"command"`.
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git commit -a -m "+ nats/command_service - Add spawn_service background task with graceful shutdown"
</suggested_git_command>

