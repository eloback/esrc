====
>   Info: Duration: 28s | ~$0.08 | Prompt Tokens: 24,144 (cached: 4,864) | Completion Tokens: 2,036 
>  Model: gpt-5.4 (gpt-5.4-2026-03-05) | Adapter: OpenAIResp
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git add -A . && git commit -m "^ consumers - add NatsStore spawning helpers for declared consumers"
</suggested_git_command>

<FILE_CHANGES>
<FILE_PATCH file_path="_workbench/consumers/plan-1-todo-steps.md">
@@
-## Step - expose ergonomic spawning helpers for automation and read model consumers
-      status: not_started
-time-created: 2026-03-26 06:00:43
-
-Add high-level helpers that make startup code concise while keeping runtime ownership in infrastructure.
-
-- Provide ergonomic methods or patterns for launching declared consumers with `NatsStore`.
-- Ensure the API keeps vertical slice code focused on declaration intent rather than transport details.
-- Align defaults with the semantics discussed in the dev chat:
-  - automations favor concurrent execution
-  - read models favor sequential execution
-
-- Keep the helpers compatible with the structured naming model based on bounded context, domain, and feature.
-
-References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
-
 ## Step - add a multi-slice example for command services and automation chaining
       status: not_started
 time-created: 2026-03-26 06:07:23
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-2-active-step.md">
@@
-## Step - integrate event_modeling declarations with NatsStore consumer startup
-      status: active
-time-created: 2026-03-26 06:00:43
-time-current: 2026-03-26 06:15:32
-
-Implement a runtime entrypoint on `NatsStore` that accepts the new event modeling consumer specification and executes it using the existing `Project` pipeline.
-
-- Add a shared consumer startup entrypoint that resolves the durable name from the structured declaration.
-- Keep durable subscription creation as an infrastructure detail.
-- Reuse a single message-processing pipeline for envelope conversion, typed context creation, projector execution, error mapping, and ack handling.
-
-- Support execution policies for:
-  - sequential processing
-  - concurrent processing with bounded in-flight work
-
-- Preserve infrastructure ownership of lifecycle concerns such as subscription creation and graceful shutdown wiring.
-
-References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
-
-### Implementation Considerations
-- Added `NatsStore::run_consumer` as the shared runtime entrypoint for `event_modeling::ConsumerSpec`.
-- Kept durable consumer creation internal to `NatsStore`, deriving the durable name and subscribed subjects from the structured declaration and projector event group.
-- Added a shared message-processing path that converts the envelope, builds typed projection context dynamically, runs the projector, maps user errors into crate `Error`, and acks successful messages.
-- Implemented execution policy support for:
-  - sequential processing that propagates failures
-  - concurrent processing with bounded in-flight work and per-message error logging
-- Added runtime validation for invalid concurrent configuration such as `max_in_flight == 0`.
-- Introduced erased projector execution support so runtime startup can operate on declared consumer specs without exposing transport details to slices.
+## Step - expose ergonomic spawning helpers for automation and read model consumers
+      status: active
+time-created: 2026-03-26 06:00:43
+time-current: 2026-03-26 06:21:27
+
+Add high-level helpers that make startup code concise while keeping runtime ownership in infrastructure.
+
+- Provide ergonomic methods or patterns for launching declared consumers with `NatsStore`.
+- Ensure the API keeps vertical slice code focused on declaration intent rather than transport details.
+- Align defaults with the semantics discussed in the dev chat:
+  - automations favor concurrent execution
+  - read models favor sequential execution
+
+- Keep the helpers compatible with the structured naming model based on bounded context, domain, and feature.
+
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
+
+### Implementation Considerations
+- Added spawning helpers on `NatsStore` that launch declared consumers onto the existing task tracker instead of requiring callers to wire `run_consumer` manually.
+- Added helper variants for `ConsumerSpec`, `Automation`, and `ReadModel` so startup code can stay at the declaration layer without transport-specific conversion boilerplate.
+- Preserved infrastructure ownership of lifecycle and error reporting by keeping task spawning and runtime failure logging inside `NatsStore`.
+- Kept the helpers compatible with the existing structured consumer naming and execution policy defaults because they delegate to `run_consumer`.
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-3-done-steps.md">
@@
 ## Step - integrate event_modeling declarations with NatsStore consumer startup
       status: done
 time-created: 2026-03-26 06:00:43
    time-done: 2026-03-26 06:15:32
@@
 - Added runtime validation for invalid concurrent configuration with a dedicated configuration error variant.
 - Introduced erased projector execution support so declared consumer specs can be executed generically by the NATS runtime layer.
+
+## Step - expose ergonomic spawning helpers for automation and read model consumers
+      status: done
+time-created: 2026-03-26 06:00:43
+   time-done: 2026-03-26 06:21:27
+
+Add high-level helpers that make startup code concise while keeping runtime ownership in infrastructure.
+
+- Provide ergonomic methods or patterns for launching declared consumers with `NatsStore`.
+- Ensure the API keeps vertical slice code focused on declaration intent rather than transport details.
+- Align defaults with the semantics discussed in the dev chat:
+  - automations favor concurrent execution
+  - read models favor sequential execution
+
+- Keep the helpers compatible with the structured naming model based on bounded context, domain, and feature.
+
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
+
+### Summary
+- Added `NatsStore` spawning helpers that launch declared consumers through the existing task tracker and runtime pipeline.
+- Added helper entrypoints for normalized `ConsumerSpec` values and for slice-facing `Automation` and `ReadModel` declarations.
+- Kept runtime ownership inside infrastructure by delegating execution to `run_consumer` and handling background task error logging within `NatsStore`.
+- Preserved the existing structured naming and execution policy behavior because the spawning helpers reuse the declaration model without redefining transport concerns.
</FILE_PATCH>

<FILE_PATCH file_path="src/nats.rs">
@@
 use std::sync::{Arc, Mutex};
 
 use async_nats::jetstream::consumer::pull::{Config as ConsumerConfig, OrderedConfig};
 use async_nats::jetstream::consumer::{Consumer, DeliverPolicy};
 use async_nats::jetstream::stream::{
     Config as StreamConfig, DiscardPolicy, Source as StreamMirror, Stream as JetStream,
 };
 use async_nats::jetstream::Context;
 use stream_cancel::Trigger;
 use tokio::sync::mpsc::{Receiver, Sender};
 use tokio_util::task::TaskTracker;
+use tracing::error;
 use tracing::instrument;
 
 use crate::error;
-use crate::event_modeling::{ConsumerSpec, ExecutionPolicy};
+use crate::event_modeling::{Automation, ConsumerSpec, ExecutionPolicy, ReadModel};
 use crate::project::DynProject;
@@
     #[instrument(skip_all, level = "debug")]
     pub async fn run_consumer<P>(&self, spec: ConsumerSpec<P>) -> error::Result<()>
     where
         P: DynProject + 'static,
     {
@@
                 self.run_consumer_concurrent(spec, stream, max_in_flight)
                     .await
             }
         }
     }
+
+    /// Spawn a declared consumer onto the store task tracker.
+    ///
+    /// This keeps runtime ownership of task lifecycle and background error
+    /// reporting inside infrastructure while letting startup code stay concise.
+    pub fn spawn_consumer<P>(&self, spec: ConsumerSpec<P>)
+    where
+        P: DynProject + 'static,
+    {
+        let store = self.clone();
+        self.graceful_shutdown.task_tracker.spawn(async move {
+            let durable_name = spec.name().durable_name();
+            if let Err(err) = store.run_consumer(spec).await {
+                error!(consumer = %durable_name, ?err, "consumer stopped");
+            }
+        });
+    }
+
+    /// Spawn an automation declaration onto the store task tracker.
+    pub fn spawn_automation<P>(&self, automation: Automation<P>)
+    where
+        P: DynProject + 'static,
+    {
+        self.spawn_consumer(automation.into_spec());
+    }
+
+    /// Spawn a read model declaration onto the store task tracker.
+    pub fn spawn_read_model<P>(&self, read_model: ReadModel<P>)
+    where
+        P: DynProject + 'static,
+    {
+        self.spawn_consumer(read_model.into_spec());
+    }
 }
</FILE_PATCH>
</FILE_CHANGES>

