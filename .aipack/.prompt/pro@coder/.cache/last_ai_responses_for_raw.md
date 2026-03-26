====
>   Info: Duration: 43s | ~$0.1909 | Prompt Tokens: 20,225 (cached: 0, cache_creation: 0) | Completion Tokens: 3,592 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 9 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.



Looking at the plan, the active step is "NATS KV-backed QueryHandler implementation" which is already implemented (see `src/nats/query_kv.rs`). So I need to move it to done and implement the next todo step: "Vertical slice helper composing ConsumerSpec and QuerySpec".

Let me analyze what's needed:
- A convenience builder that combines a `ReadModel` consumer (`ConsumerSpec`) and a `QueryHandler` (`QuerySpec`) into a single vertical slice declaration.
- A `spawn_read_model_slice` method on `NatsStore` to spawn both the consumer and query service in one call.

<FILE_CHANGES>

<FILE_PATCH file_path="_workbench/queries/plan-2-active-step.md">
@@
-# Plan 2 - Active Step
-
-## Step - NATS KV-backed QueryHandler implementation
-      status: active
-time-created: 2026-03-26 15:39:52
-time-current: 2026-03-26 16:28:37
-
-- Create a concrete `QueryHandler` implementation backed by NATS JetStream Key-Value store.
-- This provides a persistence layer for read models without requiring the developer to set up external storage.
-- Design considerations:
-  - The KV bucket name should be derived from the `ComponentName` (convention over configuration), with an option to override.
-  - `get_by_id` reads from the KV bucket by key.
-  - The `Project` implementation writes to the same KV bucket (the developer calls a save/put method inside their `project()` implementation).
-  - Provide a helper or wrapper that the developer can hold inside their `Project` impl to write to the KV store, and that also implements `QueryHandler` for reads.
-  - Serialization format: JSON (consistent with other NATS message payloads in the crate).
-- Module placement: `src/nats/query_kv.rs` or similar, registered in `src/nats.rs`.
-- References: see `src/query.rs` for `QueryHandler`, `src/nats.rs` for `NatsStore`.
+# Plan 2 - Active Step
+
+## Step - Vertical slice helper composing ConsumerSpec and QuerySpec
+      status: active
+time-created: 2026-03-26 15:39:52
+time-current: 2026-03-26 16:54:53
+
+- Create a convenience builder or type that declares a `ReadModel` consumer (`ConsumerSpec`) and its `QueryHandler` (`QuerySpec`) together as a single vertical slice.
+- This simplifies the developer experience for the common case where a read model has both an event consumer (write side) and a query handler (read side).
+- Design considerations:
+  - Should compose, not replace, the existing `ConsumerSpec` and `QuerySpec` types.
+  - A builder pattern (e.g., `ReadModelSlice::new(name, projector, handler)`) that produces both specs, or a struct that holds both and can be passed to `NatsStore` for spawning.
+  - Consider adding a `spawn_read_model_slice` or similar convenience method on `NatsStore` that spawns both the consumer and the query service in one call.
+  - Be mindful of how these components are registered in the runtime (the user emphasized developer UX and runtime registration flow).
+- Module placement: likely in `src/event_modeling.rs` since it is a declaration/composition concern, or a new `src/slice.rs` if it grows.
+- References: see `src/event_modeling.rs` for `ConsumerSpec`, `ReadModel`, `Automation`; see `src/query.rs` for `QuerySpec`.
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/queries/plan-1-todo-steps.md">
@@
 # Plan 1 - Todo Steps
 
-## Step - Vertical slice helper composing ConsumerSpec and QuerySpec
-      status: not_started
-time-created: 2026-03-26 15:39:52
-
-- Create a convenience builder or type that declares a `ReadModel` consumer (`ConsumerSpec`) and its `QueryHandler` (`QuerySpec`) together as a single vertical slice.
-- This simplifies the developer experience for the common case where a read model has both an event consumer (write side) and a query handler (read side).
-- Design considerations:
-  - Should compose, not replace, the existing `ConsumerSpec` and `QuerySpec` types.
-  - A builder pattern (e.g., `ReadModelSlice::new(name, projector, handler)`) that produces both specs, or a struct that holds both and can be passed to `NatsStore` for spawning.
-  - Consider adding a `spawn_read_model_slice` or similar convenience method on `NatsStore` that spawns both the consumer and the query service in one call.
-  - Be mindful of how these components are registered in the runtime (the user emphasized developer UX and runtime registration flow).
-- Module placement: likely in `src/event_modeling.rs` since it is a declaration/composition concern, or a new `src/slice.rs` if it grows.
-- References: see `src/event_modeling.rs` for `ConsumerSpec`, `ReadModel`, `Automation`; see `src/query.rs` for `QuerySpec`.
-
 ## Step - Caching decorator for QueryHandler
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/queries/plan-3-done-steps.md">
@@
 - Registered `pub mod query_service` in `src/nats.rs`.
+
+## Step - NATS KV-backed QueryHandler implementation
+      status: done
+time-created: 2026-03-26 15:39:52
+   time-done: 2026-03-26 16:54:53
+
+- Created `src/nats/query_kv.rs` with `NatsKvStore<RM, Q>`, a shared read/write store backed by NATS JetStream Key-Value.
+
+- `NatsKvStore` provides:
+  - `new(nats_store, name, query_fn)`: constructor deriving bucket name from `ComponentName::durable_name()` prefixed with `rm_`.
+  - `with_bucket_name`, `from_context`: alternative constructors for custom bucket names or direct JetStream context usage.
+  - `put`, `delete`, `get`: CRUD operations serializing read models as JSON.
+  - `bucket()`: access to the underlying NATS KV bucket for advanced operations.
+
+- Implements `QueryHandler` with `type Id = String`, delegating `get_by_id` to `get` and `handle` to the user-supplied closure.
+
+- Store is cheaply cloneable (`Arc`-wrapped), shareable between `Project` (write) and `QuerySpec` (read).
+
+- Registered `pub mod query_kv` in `src/nats.rs`.
</FILE_PATCH>

<FILE_APPEND file_path="src/event_modeling.rs">

/// A declaration builder that composes a read model consumer (`ConsumerSpec`)
/// and its query handler (`QuerySpec`) as a single vertical slice.
///
/// This simplifies the common case where a read model has both an event
/// consumer (write side) and a query handler (read side), allowing both to
/// be declared and spawned together.
///
/// # Example
///
/// ```rust,ignore
/// use esrc::event_modeling::{ComponentName, ReadModelSlice};
/// use esrc::query::QueryTransport;
///
/// let slice = ReadModelSlice::new(
///     ComponentName::new("shop", "catalog", "products", "list"),
///     my_projector,
///     my_kv_store,
/// );
///
/// // Spawn both the consumer and query service:
/// nats_store.spawn_read_model_slice(slice);
/// ```
#[derive(Clone, Debug)]
pub struct ReadModelSlice<P, H> {
    consumer_spec: ConsumerSpec<P>,
    query_spec: crate::query::QuerySpec<H>,
}

impl<P, H> ReadModelSlice<P, H> {
    /// Create a new read model slice with the given projector and query handler.
    ///
    /// The `ComponentName` is shared between the consumer and query specs.
    /// The consumer is created with `ConsumerRole::ReadModel` defaults
    /// (sequential execution). The query transport defaults to
    /// `NatsRequestReply`.
    pub fn new(name: ComponentName, projector: P, handler: H) -> Self {
        let consumer_spec = ConsumerSpec::new(name.clone(), ConsumerRole::ReadModel, projector);
        let query_spec = crate::query::QuerySpec::new(
            name,
            crate::query::QueryTransport::NatsRequestReply,
            handler,
        );

        Self {
            consumer_spec,
            query_spec,
        }
    }

    /// Override the execution policy for the consumer.
    pub fn with_execution_policy(mut self, execution_policy: ExecutionPolicy) -> Self {
        self.consumer_spec = self.consumer_spec.with_execution_policy(execution_policy);
        self
    }

    /// Override the transport for the query service.
    pub fn with_query_transport(mut self, transport: crate::query::QueryTransport) -> Self {
        self.query_spec = self.query_spec.with_transport(transport);
        self
    }

    /// Returns a reference to the consumer specification.
    pub fn consumer_spec(&self) -> &ConsumerSpec<P> {
        &self.consumer_spec
    }

    /// Returns a reference to the query specification.
    pub fn query_spec(&self) -> &crate::query::QuerySpec<H> {
        &self.query_spec
    }

    /// Consumes the slice and returns both specifications.
    pub fn into_specs(self) -> (ConsumerSpec<P>, crate::query::QuerySpec<H>) {
        (self.consumer_spec, self.query_spec)
    }

    /// Returns a reference to the shared component name.
    pub fn name(&self) -> &ComponentName {
        self.consumer_spec.name()
    }

    /// Returns a reference to the projector.
    pub fn projector(&self) -> &P {
        self.consumer_spec.projector()
    }

    /// Returns a mutable reference to the projector.
    pub fn projector_mut(&mut self) -> &mut P {
        self.consumer_spec.projector_mut()
    }

    /// Returns a reference to the query handler.
    pub fn handler(&self) -> &H {
        self.query_spec.handler()
    }

    /// Returns a mutable reference to the query handler.
    pub fn handler_mut(&mut self) -> &mut H {
        self.query_spec.handler_mut()
    }
}
</FILE_APPEND>

<FILE_PATCH file_path="src/nats.rs">
@@
 use crate::event_modeling::{Automation, ConsumerSpec, ExecutionPolicy, ReadModel};
 use crate::project::Project;
+use crate::query::{Query, QueryHandler, QueryService};
@@
     /// Spawn a read model declaration onto the store task tracker.
     pub fn spawn_read_model<P>(&self, read_model: ReadModel<P>)
     where
         P: Project + Send + Sync + Clone + 'static,
         P::EventGroup: crate::event::EventGroup + Send,
         P::Error: std::error::Error + Send + Sync + 'static,
     {
         self.spawn_consumer(read_model.into_spec());
     }
+
+    /// Spawn a read model slice, starting both the event consumer and the
+    /// query service as tracked background tasks.
+    ///
+    /// This is the recommended way to register a vertical slice that pairs
+    /// a read model projection with its query handler.
+    pub fn spawn_read_model_slice<P, H>(
+        &self,
+        slice: crate::event_modeling::ReadModelSlice<P, H>,
+    ) where
+        P: Project + Send + Sync + Clone + 'static,
+        P::EventGroup: crate::event::EventGroup + Send,
+        P::Error: std::error::Error + Send + Sync + 'static,
+        H: QueryHandler + Send + Sync + 'static,
+        H::Query: serde::de::DeserializeOwned + Sync,
+        H::Id: serde::de::DeserializeOwned,
+        <H::Query as Query>::ReadModel: serde::Serialize + Sync,
+        <H::Query as Query>::Response: serde::Serialize + Sync,
+    {
+        let (consumer_spec, query_spec) = slice.into_specs();
+
+        // Spawn the event consumer (read model projection).
+        self.spawn_consumer(consumer_spec);
+
+        // Spawn the query service.
+        self.spawn_query_service(query_spec);
+    }
 }
</FILE_PATCH>

<FILE_PATCH file_path="src/lib.rs">
@@
 pub use event_modeling::Automation;
+pub use event_modeling::ReadModelSlice;
</FILE_PATCH>

</FILE_CHANGES>

The implementation adds:

1. **`ReadModelSlice<P, H>`** in `src/event_modeling.rs`: A builder struct that composes a `ConsumerSpec` (read model consumer) and a `QuerySpec` (query handler) using a shared `ComponentName`. It provides:
   - `new(name, projector, handler)` constructor with sensible defaults (sequential execution, NATS request-reply transport).
   - Override methods for execution policy and query transport.
   - Accessors for both specs, projector, handler, and a destructuring `into_specs()`.

2. **`spawn_read_model_slice`** on `NatsStore` in `src/nats.rs`: A single method that takes a `ReadModelSlice`, destructures it into its two specs, and spawns both the event consumer and the query service as tracked background tasks.

3. **Re-export** of `ReadModelSlice` from `src/lib.rs` alongside the existing `Automation` re-export.

<suggested_git_command>
git add -A . && git commit -a -m "+ event_modeling - Add ReadModelSlice vertical slice helper composing ConsumerSpec and QuerySpec"
</suggested_git_command>

