====
>   Info: Duration: 46s | ~$0.1125 | Prompt Tokens: 28,178 (cached: 5,888) | Completion Tokens: 3,690 
>  Model: gpt-5.4 (gpt-5.4-2026-03-05) | Adapter: OpenAIResp
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git commit -a -m "^ consumers - replace DynProject runtime path with generic projector execution"
</suggested_git_command>

<FILE_CHANGES>
<FILE_PATCH file_path="src/event_modeling.rs">
@@
-use crate::project::DynProject;
-
 /// The semantic role a consumer plays in the system.
@@
 impl<P> ConsumerSpec<P>
-where
-    P: DynProject,
 {
@@
 impl<P> Automation<P>
-where
-    P: DynProject,
 {
@@
 impl<P> ReadModel<P>
-where
-    P: DynProject,
 {
</FILE_PATCH>

<FILE_PATCH file_path="src/project.rs">
@@
 /// A type-safe wrapper for the deserialized contents of an [`Envelope`].
@@
 pub struct Context<'de, E, G> {
     envelope: &'de E,
     event_group: G,
 }
@@
-/// An object-safe runner for erased consumer execution.
-pub trait DynProject: Send + Sync {
-    /// The event group declaration for this projector.
-    fn event_group(&self) -> &dyn event::EventGroupType;
-
-    /// Clone this projector as a boxed trait object.
-    fn clone_box(&self) -> Box<dyn DynProject>;
-
-    /// Apply a received event, triggering implementation specific behavior.
-    fn project_boxed<'de, 'a, E>(
-        &'a mut self,
-        context: Context<'de, E, Self::EventGroup>,
-    ) -> impl std::future::Future<Output = error::Result<()>> + Send + 'a
-    where
-        E: Envelope + Sync + 'de,
-        Self: Sized;
-
-    /// Apply a received event using dynamic dispatch.
-    fn project_dyn<'de, 'a, E>(
-        &'a mut self,
-        envelope: &'de E,
-    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<()>> + Send + 'a>>
-    where
-        E: Envelope + Sync + 'de + 'a;
-}
-
-impl Clone for Box<dyn DynProject> {
-    fn clone(&self) -> Self {
-        self.clone_box()
-    }
-}
-
-impl<P> DynProject for P
-where
-    P: for<'de> Project<
-            EventGroup = <P as Project>::EventGroup,
-            Error = <P as Project>::Error,
-        > + Send
-        + Sync
-        + Clone
-        + 'static,
-    P::EventGroup: event::EventGroupType + TryFromEnvelope + Send,
-    P::Error: std::error::Error + Send + Sync + 'static,
-{
-    fn event_group(&self) -> &dyn event::EventGroupType {
-        use std::sync::OnceLock;
-
-        struct Holder<G>(std::marker::PhantomData<G>);
-
-        impl<G> Default for Holder<G> {
-            fn default() -> Self {
-                Self(std::marker::PhantomData)
-            }
-        }
-
-        impl<G> event::EventGroupType for Holder<G>
-        where
-            G: EventGroup + Send + Sync + 'static,
-        {
-            fn names(&self) -> Vec<&'static str> {
-                G::names().collect()
-            }
-        }
-
-        static EVENT_GROUP: OnceLock<Holder<P::EventGroup>> = OnceLock::new();
-        EVENT_GROUP.get_or_init(Holder::default)
-    }
-
-    fn clone_box(&self) -> Box<dyn DynProject> {
-        Box::new(self.clone())
-    }
-
-    fn project_boxed<'de, 'a, E>(
-        &'a mut self,
-        context: Context<'de, E, Self::EventGroup>,
-    ) -> impl std::future::Future<Output = error::Result<()>> + Send + 'a
-    where
-        E: Envelope + Sync + 'de,
-        Self: Sized,
-    {
-        async move {
-            Project::project(self, context)
-                .await
-                .map_err(|e| error::Error::External(e.into()))
-        }
-    }
-
-    fn project_dyn<'de, 'a, E>(
-        &'a mut self,
-        envelope: &'de E,
-    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<()>> + Send + 'a>>
-    where
-        E: Envelope + Sync + 'de + 'a,
-    {
-        Box::pin(async move {
-            let context = Context::<E, P::EventGroup>::try_with_envelope(envelope)?;
-            Project::project(self, context)
-                .await
-                .map_err(|e| error::Error::External(e.into()))
-        })
-    }
-}
-
 impl<'de, E, G> Context<'de, E, G>
 where
     E: Envelope,
     G: EventGroup + TryFromEnvelope,
 {
</FILE_PATCH>

<FILE_PATCH file_path="src/nats.rs">
@@
 use tracing::error;
 use tracing::instrument;
 
 use crate::error;
 use crate::event_modeling::{Automation, ConsumerSpec, ExecutionPolicy, ReadModel};
-use crate::project::DynProject;
+use crate::project::Project;
@@
     pub async fn run_consumer<P>(&self, spec: ConsumerSpec<P>) -> error::Result<()>
     where
-        P: DynProject + 'static,
+        P: for<'de> Project<
+                EventGroup = <P as Project>::EventGroup,
+                Error = <P as Project>::Error,
+            > + Send
+            + Sync
+            + Clone
+            + 'static,
+        P::EventGroup: crate::event::EventGroup + Send,
+        P::Error: std::error::Error + Send + Sync + 'static,
     {
         let durable_name = spec.name().durable_name();
-        let mut names = spec.projector().event_group().names();
+        let mut names = <P::EventGroup as crate::event::EventGroup>::names().collect::<Vec<_>>();
         names.sort();
@@
     pub fn spawn_consumer<P>(&self, spec: ConsumerSpec<P>)
     where
-        P: DynProject + 'static,
+        P: for<'de> Project<
+                EventGroup = <P as Project>::EventGroup,
+                Error = <P as Project>::Error,
+            > + Send
+            + Sync
+            + Clone
+            + 'static,
+        P::EventGroup: crate::event::EventGroup + Send,
+        P::Error: std::error::Error + Send + Sync + 'static,
     {
@@
     pub fn spawn_automation<P>(&self, automation: Automation<P>)
     where
-        P: DynProject + 'static,
+        P: for<'de> Project<
+                EventGroup = <P as Project>::EventGroup,
+                Error = <P as Project>::Error,
+            > + Send
+            + Sync
+            + Clone
+            + 'static,
+        P::EventGroup: crate::event::EventGroup + Send,
+        P::Error: std::error::Error + Send + Sync + 'static,
     {
@@
     pub fn spawn_read_model<P>(&self, read_model: ReadModel<P>)
     where
-        P: DynProject + 'static,
+        P: for<'de> Project<
+                EventGroup = <P as Project>::EventGroup,
+                Error = <P as Project>::Error,
+            > + Send
+            + Sync
+            + Clone
+            + 'static,
+        P::EventGroup: crate::event::EventGroup + Send,
+        P::Error: std::error::Error + Send + Sync + 'static,
     {
         self.spawn_consumer(read_model.into_spec());
     }
 }
</FILE_PATCH>

<FILE_PATCH file_path="src/nats/event.rs">
@@
 use crate::error::{self, Error};
 use crate::event::{Event, EventGroup, Publish, Replay, ReplayOne, Sequence, Subscribe, Truncate};
 use crate::event_modeling::ConsumerSpec;
-use crate::project::DynProject;
+use crate::project::{Context, Project};
 use crate::version::SerializeVersion;
@@
 impl NatsStore {
-    async fn process_consumer_message(
-        mut projector: Box<dyn DynProject>,
-        message: error::Result<NatsEnvelope>,
-    ) -> error::Result<()> {
+    async fn process_consumer_message<P>(
+        projector: &mut P,
+        message: error::Result<NatsEnvelope>,
+    ) -> error::Result<()>
+    where
+        P: for<'de> Project<
+                EventGroup = <P as Project>::EventGroup,
+                Error = <P as Project>::Error,
+            > + Send
+            + Sync,
+        P::EventGroup: EventGroup + Send,
+        P::Error: std::error::Error + Send + Sync + 'static,
+    {
         let envelope = message?;
-        projector.project_dyn(&envelope).await?;
+        let context = Context::<NatsEnvelope, P::EventGroup>::try_with_envelope(&envelope)?;
+        Project::project(projector, context)
+            .await
+            .map_err(|e| Error::External(e.into()))?;
         envelope.ack().await;
         Ok(())
     }
 
     pub(crate) async fn run_consumer_sequential<S, P>(
@@
     ) -> error::Result<()>
     where
         S: Stream<Item = error::Result<NatsEnvelope>> + Send + Unpin,
-        P: DynProject + 'static,
+        P: for<'de> Project<
+                EventGroup = <P as Project>::EventGroup,
+                Error = <P as Project>::Error,
+            > + Send
+            + Sync
+            + Clone
+            + 'static,
+        P::EventGroup: EventGroup + Send,
+        P::Error: std::error::Error + Send + Sync + 'static,
     {
+        let mut projector = spec.into_projector();
         while let Some(message) = stream.next().await {
-            Self::process_consumer_message(Box::new(spec.projector().clone()), message).await?;
+            Self::process_consumer_message(&mut projector, message).await?;
         }
 
         Ok(())
@@
     ) -> error::Result<()>
     where
         S: Stream<Item = error::Result<NatsEnvelope>> + Send,
-        P: DynProject + 'static,
+        P: for<'de> Project<
+                EventGroup = <P as Project>::EventGroup,
+                Error = <P as Project>::Error,
+            > + Send
+            + Sync
+            + Clone
+            + 'static,
+        P::EventGroup: EventGroup + Send,
+        P::Error: std::error::Error + Send + Sync + 'static,
     {
         let durable_name = spec.name().durable_name();
 
         stream
             .for_each_concurrent(Some(max_in_flight), move |message| {
                 let durable_name = durable_name.clone();
-                let projector = Box::new(spec.projector().clone()) as Box<dyn DynProject>;
+                let mut projector = spec.projector().clone();
 
                 async move {
-                    if let Err(error) = Self::process_consumer_message(projector, message).await {
+                    if let Err(error) = Self::process_consumer_message(&mut projector, message).await {
                         tracing::error!(consumer = %durable_name, ?error, "Error processing message");
                     }
                 }
             })
             .await;
@@
     use crate::{
         event::future::IntoSendFuture,
-        project::{Context, Project},
+        project::{Context, Project},
     };
@@
         ) -> error::Result<()>
         where
             P: Project,
         {
*** End Patch
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-1-todo-steps.md">
@@
-## Step - implement the new projector execution abstraction and remove DynProject compile errors
-      status: not_started
-time-created: 2026-03-26 07:00:52
-
-Replace the current `DynProject` implementation with the compile-safe abstraction defined in the previous step, updating the relevant runtime and declaration code so the crate builds correctly again.
-
-- Update the projector execution flow used by consumer declarations and `NatsStore` runtime helpers.
-
-- Ensure the implementation preserves the existing declaration-layer ergonomics where possible, while fixing object-safety, associated type, and async dispatch problems in the current design.
-
-- Remove or adapt the existing `DynProject` machinery only as needed to support the new execution model cleanly.
-
-- References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.
-
 ## Step - validate consumer declaration integration and document the updated projector model
       status: not_started
 time-created: 2026-03-26 07:00:52
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-2-active-step.md">
@@
-## Step - define the DynProject replacement direction and compile-safe abstraction
+## Step - implement the new projector execution abstraction and remove DynProject compile errors
       status: active
 time-created: 2026-03-26 07:00:52
-time-current: 2026-03-26 07:04:25
+time-current: 2026-03-26 07:07:19
 
-Document the intended replacement direction for the current `DynProject` design, focusing on a compile-safe abstraction that better matches the projector execution domain.
+Replace the current `DynProject` implementation with the compile-safe abstraction defined in the previous step, updating the relevant runtime and declaration code so the crate builds correctly again.
 
-- Evaluate whether the runtime should use boxed projector values directly, a new object-safe trait tailored to consumer execution, or another erased projector model that avoids the current generic/object-safety issues.
+- Update the projector execution flow used by consumer declarations and `NatsStore` runtime helpers.
 
-- Define the responsibilities and constraints for the replacement abstraction, including:
-  - clone or factory behavior for per-message or per-task projector usage
-  - access to the event group names needed by consumer startup
-  - async message handling compatibility with the typed `Context` flow
-  - compatibility with sequential and concurrent NATS consumer execution
+- Ensure the implementation preserves the existing declaration-layer ergonomics where possible, while fixing object-safety, associated type, and async dispatch problems in the current design.
 
-- Capture the chosen direction in a way that subsequent implementation steps can reference without ambiguity.
+- Remove or adapt the existing `DynProject` machinery only as needed to support the new execution model cleanly.
 
 References: see the retained event modeling design context in `plan-3-done-steps.md`, step `Step - integrate event_modeling declarations with NatsStore consumer startup`.
 
 ### Implementation Considerations
@@
 - Expected code direction in the next step:
   - `event_modeling` should depend on the declaration shape only, not on a dynamic projector trait.
   - `NatsStore` should become the owner of the generic execution logic for `P: Project + Clone + Send + Sync + 'static`.
   - `project.rs` should retain `Context` and `Project`, while removing or adapting only the `DynProject` machinery that caused the compile-safety issues.
+
+### sub-step - replace dynproject runtime usage with generic projector execution
+time-current: 2026-03-26 07:07:19
+
+#### User ask
+
+following the plan rules, implement the next step of the plan.
+
+#### AI Answer
+
+Implemented the runtime shift away from `DynProject` by making the NATS consumer execution path generic over concrete `Project` types.
+
+- Removed the `DynProject` dependency from declaration construction in `event_modeling`.
+
+- Removed the `DynProject` trait machinery from `project.rs`, while preserving `Context` and `Project`.
+
+- Updated `NatsStore` startup and consumer execution helpers to:
+  - derive event names from `P::EventGroup`
+  - keep a single mutable projector for sequential execution
+  - clone a projector per in-flight task for concurrent execution
+  - build typed `Context` values directly from `NatsEnvelope`
+
+- Preserved the declaration-layer API shape of `ConsumerSpec<P>`, `Automation<P>`, and `ReadModel<P>`, while moving execution concerns fully into generic runtime helpers.
 </FILE_PATCH>
</FILE_CHANGES>

