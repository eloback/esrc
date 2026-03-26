====
>   Info: Duration: 46s | ~$0.1072 | Prompt Tokens: 23,131 (cached: 4,864) | Completion Tokens: 4,018 
>  Model: gpt-5.4 (gpt-5.4-2026-03-05) | Adapter: OpenAIResp
>  Files: Context Files: 12 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git add -A . && git commit -m "^ consumers - integrate event_modeling specs with NatsStore runtime"
</suggested_git_command>

<FILE_CHANGES>
<FILE_PATCH file_path="src/error.rs">
@@
     /// An optimistic concurrency error occurred.
     ///
     /// This happens when trying to publish an Event with a last sequence number
     /// that is no longer the most recent sequence number. Calls in this case
     /// can usually just be retried after applying any new events.
     #[error("event transaction failed")]
     Conflict,
+    /// An invalid consumer execution policy was provided.
+    ///
+    /// This is used for runtime configuration errors such as a concurrent
+    /// consumer declaration with zero maximum in-flight work.
+    #[error("invalid consumer configuration ({0})")]
+    Configuration(String),
 }
</FILE_PATCH>

<FILE_PATCH file_path="src/event.rs">
@@
 pub trait Event: Send {
@@
 }
 
 /// Reference multiple event types in a single read stream.
@@
 pub trait EventGroup {
     /// The names of all events within this Group.
     fn names() -> impl Iterator<Item = &'static str>;
 }
+
+/// A helper trait for erased execution over event groups.
+pub trait EventGroupType: Send + Sync + 'static {
+    /// The names of all events within this Group.
+    fn names(&self) -> Vec<&'static str>;
+}
@@
 impl<E: Event> EventGroup for E {
     fn names() -> impl Iterator<Item = &'static str> {
         iter::once(E::name())
     }
 }
+
+impl<G> EventGroupType for G
+where
+    G: EventGroup + Send + Sync + 'static,
+{
+    fn names(&self) -> Vec<&'static str> {
+        G::names().collect()
+    }
+}
</FILE_PATCH>

<FILE_PATCH file_path="src/project.rs">
@@
 use crate::envelope::{Envelope, TryFromEnvelope};
 use crate::event::{self, Sequence};
 use crate::{error, EventGroup};
@@
 pub trait Project: Send + Sync + Clone {
@@
     where
         E: Envelope + Sync;
 }
+
+/// An object-safe runner for erased consumer execution.
+pub trait DynProject: Send + Sync {
+    /// The event group declaration for this projector.
+    fn event_group(&self) -> &dyn event::EventGroupType;
+
+    /// Clone this projector as a boxed trait object.
+    fn clone_box(&self) -> Box<dyn DynProject>;
+
+    /// Apply a received event, triggering implementation specific behavior.
+    fn project_boxed<'de, 'a, E>(
+        &'a mut self,
+        context: Context<'de, E, Self::EventGroup>,
+    ) -> impl std::future::Future<Output = error::Result<()>> + Send + 'a
+    where
+        E: Envelope + Sync + 'de,
+        Self: Sized;
+
+    /// Apply a received event using dynamic dispatch.
+    fn project_dyn<'de, 'a, E>(
+        &'a mut self,
+        envelope: &'de E,
+    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<()>> + Send + 'a>>
+    where
+        E: Envelope + Sync + 'de + 'a;
+}
+
+impl Clone for Box<dyn DynProject> {
+    fn clone(&self) -> Self {
+        self.clone_box()
+    }
+}
+
+impl<P> DynProject for P
+where
+    P: for<'de> Project<
+            EventGroup = <P as Project>::EventGroup,
+            Error = <P as Project>::Error,
+        > + Send
+        + Sync
+        + Clone
+        + 'static,
+    P::EventGroup: event::EventGroupType + TryFromEnvelope + Send,
+    P::Error: std::error::Error + Send + Sync + 'static,
+{
+    fn event_group(&self) -> &dyn event::EventGroupType {
+        use std::sync::OnceLock;
+
+        struct Holder<G>(std::marker::PhantomData<G>);
+
+        impl<G> Default for Holder<G> {
+            fn default() -> Self {
+                Self(std::marker::PhantomData)
+            }
+        }
+
+        impl<G> event::EventGroupType for Holder<G>
+        where
+            G: EventGroup + Send + Sync + 'static,
+        {
+            fn names(&self) -> Vec<&'static str> {
+                G::names().collect()
+            }
+        }
+
+        static EVENT_GROUP: OnceLock<Holder<P::EventGroup>> = OnceLock::new();
+        EVENT_GROUP.get_or_init(Holder::default)
+    }
+
+    fn clone_box(&self) -> Box<dyn DynProject> {
+        Box::new(self.clone())
+    }
+
+    fn project_boxed<'de, 'a, E>(
+        &'a mut self,
+        context: Context<'de, E, Self::EventGroup>,
+    ) -> impl std::future::Future<Output = error::Result<()>> + Send + 'a
+    where
+        E: Envelope + Sync + 'de,
+        Self: Sized,
+    {
+        async move {
+            Project::project(self, context)
+                .await
+                .map_err(|e| error::Error::External(e.into()))
+        }
+    }
+
+    fn project_dyn<'de, 'a, E>(
+        &'a mut self,
+        envelope: &'de E,
+    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<()>> + Send + 'a>>
+    where
+        E: Envelope + Sync + 'de + 'a,
+    {
+        Box::pin(async move {
+            let context = Context::<E, P::EventGroup>::try_with_envelope(envelope)?;
+            Project::project(self, context)
+                .await
+                .map_err(|e| error::Error::External(e.into()))
+        })
+    }
+}
 </FILE_PATCH>

<FILE_PATCH file_path="src/event_modeling.rs">
@@
-use crate::project::Project;
+use crate::project::DynProject;
@@
 pub struct ConsumerSpec<P> {
@@
 }
@@
 impl<P> ConsumerSpec<P>
 where
-    P: Project,
+    P: DynProject,
 {
@@
 pub struct Automation<P> {
@@
 impl<P> Automation<P>
 where
-    P: Project,
+    P: DynProject,
 {
@@
 pub struct ReadModel<P> {
@@
 impl<P> ReadModel<P>
 where
-    P: Project,
+    P: DynProject,
 {
*** End Patch
</FILE_PATCH>

<FILE_PATCH file_path="src/nats.rs">
@@
 use tracing::instrument;
 
 use crate::error;
+use crate::event_modeling::{ConsumerSpec, ExecutionPolicy};
+use crate::project::DynProject;
@@
     async fn durable_consumer(
         &self,
         name: String,
         subjects: Vec<String>,
     ) -> error::Result<Consumer<ConsumerConfig>> {
@@
 
         Ok(self.stream.create_consumer(config).await?)
     }
+
+    #[instrument(skip_all, level = "debug")]
+    pub async fn run_consumer<P>(&self, spec: ConsumerSpec<P>) -> error::Result<()>
+    where
+        P: DynProject + 'static,
+    {
+        let durable_name = spec.name().durable_name();
+        let mut names = spec.projector().event_group().names();
+        names.sort();
+
+        let subjects = names
+            .iter()
+            .map(|&n| NatsSubject::Event(n.into()).into_string(self.prefix))
+            .collect();
+
+        let consumer = self.durable_consumer(durable_name, subjects).await?;
+        let stream = consumer
+            .messages()
+            .await?
+            .map(|m| NatsEnvelope::try_from_message(self.prefix, m?));
+
+        match spec.execution_policy() {
+            ExecutionPolicy::Sequential => self.run_consumer_sequential(spec, stream).await,
+            ExecutionPolicy::Concurrent { max_in_flight } => {
+                if max_in_flight == 0 {
+                    return Err(error::Error::Configuration(
+                        "concurrent consumers require max_in_flight > 0".to_owned(),
+                    ));
+                }
+
+                self.run_consumer_concurrent(spec, stream, max_in_flight)
+                    .await
+            }
+        }
+    }
 }
 </FILE_PATCH>

<FILE_PATCH file_path="src/nats/event.rs">
@@
 use futures::{Stream, StreamExt};
 use tracing::instrument;
 use uuid::Uuid;
 
 use super::header::{EVENT_TYPE, METADATA_PREFIX, VERSION_KEY};
 use super::subject::NatsSubject;
 use super::{NatsEnvelope, NatsStore};
 use crate::error::{self, Error};
 use crate::event::{Event, EventGroup, Publish, Replay, ReplayOne, Sequence, Subscribe, Truncate};
+use crate::event_modeling::ConsumerSpec;
+use crate::project::DynProject;
 use crate::version::SerializeVersion;
@@
 pub mod custom {
@@
 }
+
+impl NatsStore {
+    async fn process_consumer_message(
+        mut projector: Box<dyn DynProject>,
+        message: error::Result<NatsEnvelope>,
+    ) -> error::Result<()> {
+        let envelope = message?;
+        projector.project_dyn(&envelope).await?;
+        envelope.ack().await;
+        Ok(())
+    }
+
+    pub(crate) async fn run_consumer_sequential<S, P>(
+        &self,
+        spec: ConsumerSpec<P>,
+        mut stream: S,
+    ) -> error::Result<()>
+    where
+        S: Stream<Item = error::Result<NatsEnvelope>> + Send + Unpin,
+        P: DynProject + 'static,
+    {
+        while let Some(message) = stream.next().await {
+            Self::process_consumer_message(Box::new(spec.projector().clone()), message).await?;
+        }
+
+        Ok(())
+    }
+
+    pub(crate) async fn run_consumer_concurrent<S, P>(
+        &self,
+        spec: ConsumerSpec<P>,
+        stream: S,
+        max_in_flight: usize,
+    ) -> error::Result<()>
+    where
+        S: Stream<Item = error::Result<NatsEnvelope>> + Send,
+        P: DynProject + 'static,
+    {
+        let durable_name = spec.name().durable_name();
+
+        stream
+            .for_each_concurrent(Some(max_in_flight), move |message| {
+                let durable_name = durable_name.clone();
+                let projector = Box::new(spec.projector().clone()) as Box<dyn DynProject>;
+
+                async move {
+                    if let Err(error) = Self::process_consumer_message(projector, message).await {
+                        tracing::error!(consumer = %durable_name, ?error, "Error processing message");
+                    }
+                }
+            })
+            .await;
+
+        Ok(())
+    }
+}
 
 impl Replay for NatsStore {
     type Envelope = NatsEnvelope;
 </FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-1-todo-steps.md">
@@
-## Step - integrate event_modeling declarations with NatsStore consumer startup
-      status: not_started
-time-created: 2026-03-26 06:00:43
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
-- References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
-
 ## Step - expose ergonomic spawning helpers for automation and read model consumers
       status: not_started
 time-created: 2026-03-26 06:00:43
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-2-active-step.md">
@@
-## Step - implement the event_modeling module with declaration types and builders
-      status: active
+## Step - integrate event_modeling declarations with NatsStore consumer startup
+      status: active
 time-created: 2026-03-26 06:00:43
-time-current: 2026-03-26 06:12:05
+time-current: 2026-03-26 06:15:32
 
-Add the new `event_modeling` module and implement the declaration-facing types needed by vertical slices.
+Implement a runtime entrypoint on `NatsStore` that accepts the new event modeling consumer specification and executes it using the existing `Project` pipeline.
 
-- Create the module files and expose them from the crate.
-- Implement the declaration model planned in the previous step, including:
-  - consumer role representation
-  - execution policy representation
-  - structured consumer naming using bounded context, domain, and feature
-  - normalized consumer specification
-  - ergonomic `Automation` and `ReadModel` builders
+- Add a shared consumer startup entrypoint that resolves the durable name from the structured declaration.
+- Keep durable subscription creation as an infrastructure detail.
+- Reuse a single message-processing pipeline for envelope conversion, typed context creation, projector execution, error mapping, and ack handling.
 
-- Keep the implementation focused on declaration and normalization primitives, without wiring runtime execution yet.
+- Support execution policies for:
+  - sequential processing
+  - concurrent processing with bounded in-flight work
+
+- Preserve infrastructure ownership of lifecycle concerns such as subscription creation and graceful shutdown wiring.
 
 References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
 
 ### Implementation Considerations
-- Implemented the declaration-focused `src/event_modeling.rs` module and exposed it from `src/lib.rs`.
-- Added the initial declaration types:
-  - `ConsumerRole`
-  - `ExecutionPolicy`
-  - `ConsumerName`
-  - `ConsumerSpec<P>`
-- Added explicit slice-facing builders:
-  - `Automation<P>`
-  - `ReadModel<P>`
-- Kept the implementation focused on declaration and normalization primitives only, with no runtime wiring in this step.
-- Preserved the structured naming model through durable name and slice path helpers derived from `bounded_context`, `domain`, `feature`, and `consumer`.
+- Added `NatsStore::run_consumer` as the shared runtime entrypoint for `event_modeling::ConsumerSpec`.
+- Kept durable consumer creation internal to `NatsStore`, deriving the durable name and subscribed subjects from the structured declaration and projector event group.
+- Added a shared message-processing path that converts the envelope, builds typed projection context dynamically, runs the projector, maps user errors into crate `Error`, and acks successful messages.
+- Implemented execution policy support for:
+  - sequential processing that propagates failures
+  - concurrent processing with bounded in-flight work and per-message error logging
+- Added runtime validation for invalid concurrent configuration such as `max_in_flight == 0`.
+- Introduced erased projector execution support so runtime startup can operate on declared consumer specs without exposing transport details to slices.
 
 </FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-3-done-steps.md">
@@
 ## Step - implement the event_modeling module with declaration types and builders
       status: done
 time-created: 2026-03-26 06:00:43
    time-done: 2026-03-26 06:12:05
@@
 - Implemented explicit `Automation<P>` and `ReadModel<P>` builders that normalize into `ConsumerSpec<P>`.
 - Added structured naming helpers for both stable durable consumer names and slice path representation.
 - Kept the step scoped to declaration and normalization primitives, with no runtime startup integration yet.
+
+## Step - integrate event_modeling declarations with NatsStore consumer startup
+      status: done
+time-created: 2026-03-26 06:00:43
+   time-done: 2026-03-26 06:15:32
+
+Implement a runtime entrypoint on `NatsStore` that accepts the new event modeling consumer specification and executes it using the existing `Project` pipeline.
+
+- Add a shared consumer startup entrypoint that resolves the durable name from the structured declaration.
+- Keep durable subscription creation as an infrastructure detail.
+- Reuse a single message-processing pipeline for envelope conversion, typed context creation, projector execution, error mapping, and ack handling.
+
+- Support execution policies for:
+  - sequential processing
+  - concurrent processing with bounded in-flight work
+
+- Preserve infrastructure ownership of lifecycle concerns such as subscription creation and graceful shutdown wiring.
+
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
+
+### Summary
+- Added `NatsStore::run_consumer` as the runtime entrypoint that accepts `event_modeling::ConsumerSpec<P>`.
+- Kept durable subscription setup inside infrastructure by deriving durable names and event subjects from the structured declaration and projector event group.
+- Reused a shared message-processing pipeline for dynamic typed context creation, projector execution, error mapping, and ack handling.
+- Implemented execution policy support for sequential and bounded concurrent processing.
+- Added runtime validation for invalid concurrent configuration with a dedicated configuration error variant.
+- Introduced erased projector execution support so declared consumer specs can be executed generically by the NATS runtime layer.
 </FILE_PATCH>
</FILE_CHANGES>

