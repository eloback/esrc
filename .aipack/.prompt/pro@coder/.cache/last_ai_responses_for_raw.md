====
>   Info: Duration: 37s | ~$0.2324 | Prompt Tokens: 31,019 (cached: 0, cache_creation: 0) | Completion Tokens: 3,093 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the plan, the active step "Create src/query.rs with Query, QueryHandler, QueryTransport, and QuerySpec" is already implemented and active. So I need to:

1. Move the active step to done.
2. Activate the next todo step "Add QueryService and QueryClient traits to src/query.rs" and implement it.

Let me implement the `QueryService` and `QueryClient` traits following the `CommandService`/`CommandClient` pattern from `src/event/command_service.rs`.

<FILE_CHANGES>

<FILE_PATCH file_path="_workbench/queries/plan-2-active-step.md">
@@
-# Plan 2 - Active Step
-
-## Step - Create src/query.rs with Query, QueryHandler, QueryTransport, and QuerySpec
-      status: active
-time-created: 2026-03-26 14:58:37
-time-current: 2026-03-26 15:08:42
-
-- Created `src/query.rs` as a single file containing:
-
-- `Query` trait:
-  - `type ReadModel: Send`
-  - `type Response: Send`
-  - No serde bounds.
-
-- `QueryHandler` trait (using `#[trait_variant::make(Send)]`):
-  - `type Query: Query`
-  - `type Id: Send + Sync`
-  - `async fn get_by_id(&self, id: Self::Id) -> crate::error::Result<Option<<Self::Query as Query>::ReadModel>>`
-  - `async fn handle(&self, query: Self::Query) -> crate::error::Result<<Self::Query as Query>::Response>`
-  - Requires `Send + Sync`, no `Clone`.
-
-- `QueryTransport` enum:
-  - `NatsRequestReply` variant (only one for now, extensible).
-
-- `QuerySpec` struct:
-  - Holds a `ComponentName` (from `event_modeling`), a `QueryTransport`, and the `QueryHandler` instance.
-  - Constructor and accessor methods following the `ConsumerSpec` pattern.
-
-- Registered the module in `src/lib.rs` with a doc comment.
+# Plan 2 - Active Step
+
+## Step - Add QueryService and QueryClient traits to src/query.rs
+      status: active
+time-created: 2026-03-26 14:58:37
+time-current: 2026-03-26 15:15:09
+
+- Added transport-agnostic trait definitions to `src/query.rs`, following the `CommandService`/`CommandClient` pattern from `src/event/command_service.rs`.
+
+- `QueryService` trait (using `#[trait_variant::make(Send)]`):
+  - `async fn serve<H>(&self, spec: &QuerySpec<H>) -> error::Result<()>` method to serve/handle incoming query requests.
+  - Serde bounds (`Serialize + DeserializeOwned`) added at the method level on the query enum, response, and read model types.
+  - The `QueryHandler::Id` also requires `DeserializeOwned` for receiving `get_by_id` requests.
+
+- `QueryClient` trait (using `#[trait_variant::make(Send)]`):
+  - `async fn get_by_id<Q, Id>(&self, name: &ComponentName, id: Id) -> error::Result<Option<<Q as Query>::ReadModel>>` for the built-in by-ID query.
+  - `async fn query<Q>(&self, name: &ComponentName, query: Q) -> error::Result<Q::Response>` for custom queries.
+  - Serde bounds at the method level.
+
+- Both traits use `#[trait_variant::make(Send)]`.
+- Subject derivation convention: `query.<bounded_context>.<domain>.<feature>.<component>` (documented via `ComponentName::query_subject()` helper method on `ComponentName`).
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/queries/plan-3-done-steps.md">
@@
+
+## Step - Create src/query.rs with Query, QueryHandler, QueryTransport, and QuerySpec
+      status: done
+time-created: 2026-03-26 14:58:37
+   time-done: 2026-03-26 15:15:09
+
+- Created `src/query.rs` as a single file containing:
+
+- `Query` trait:
+  - `type ReadModel: Send`
+  - `type Response: Send`
+  - No serde bounds.
+
+- `QueryHandler` trait (using `#[trait_variant::make(Send)]`):
+  - `type Query: Query`
+  - `type Id: Send + Sync`
+  - `async fn get_by_id(&self, id: Self::Id) -> crate::error::Result<Option<<Self::Query as Query>::ReadModel>>`
+  - `async fn handle(&self, query: Self::Query) -> crate::error::Result<<Self::Query as Query>::Response>`
+  - Requires `Send + Sync`, no `Clone`.
+
+- `QueryTransport` enum:
+  - `NatsRequestReply` variant (only one for now, extensible).
+
+- `QuerySpec` struct:
+  - Holds a `ComponentName` (from `event_modeling`), a `QueryTransport`, and the `QueryHandler` instance.
+  - Constructor and accessor methods following the `ConsumerSpec` pattern.
+
+- Registered the module in `src/lib.rs` with a doc comment.
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/queries/plan-1-todo-steps.md">
@@
 # Plan 1 - Todo Steps
 
-## Step - Create src/query.rs with Query, QueryHandler, QueryTransport, and QuerySpec
-      status: not_started
-time-created: 2026-03-26 14:58:37
-
-- Create `src/query.rs` as a single file containing:
-
-- `Query` trait:
-  - `type ReadModel: Send`
-  - `type Response: Send`
-  - No serde bounds.
-
-- `QueryHandler` trait (using `#[trait_variant::make(Send)]`):
-  - `type Query: Query`
-  - `type Id: Send + Sync`
-  - `async fn get_by_id(&self, id: Self::Id) -> crate::error::Result<Option<<Self::Query as Query>::ReadModel>>`
-  - `async fn handle(&self, query: Self::Query) -> crate::error::Result<<Self::Query as Query>::Response>`
-  - Requires `Send + Sync`, no `Clone`.
-
-- `QueryTransport` enum:
-  - `NatsRequestReply` variant (only one for now, extensible).
-
-- `QuerySpec` struct:
-  - Holds a `ComponentName` (from `event_modeling`), a `QueryTransport`, and the `QueryHandler` instance.
-  - Constructor and accessor methods following the `ConsumerSpec` pattern.
-
-- Register the module in `src/lib.rs` with a doc comment.
-
-## Step - Add QueryService and QueryClient traits to src/query.rs
-      status: not_started
-time-created: 2026-03-26 14:58:37
-
-- Add transport-agnostic trait definitions to `src/query.rs`, following the `CommandService`/`CommandClient` pattern from `src/event/command_service.rs`.
-
-- `QueryService` trait:
-  - Method to serve/handle incoming query requests.
-  - Serde bounds (`Serialize + DeserializeOwned`) added at the method level on the query enum and response types.
-
-- `QueryClient` trait:
-  - Method to send a query remotely and await the response.
-  - Serde bounds at the method level.
-
-- Both traits use `#[trait_variant::make(Send)]`.
-- Subject derivation convention documented: `query.<bounded_context>.<domain>.<feature>.<component>`.
-
 ## Step - Implement NATS QueryService and QueryClient
</FILE_PATCH>

<FILE_PATCH file_path="src/query.rs">
@@
 //! Traits and types for declaring and handling queries against read models.
 
-use crate::event_modeling::ComponentName;
+use std::future::Future;
+
+use serde::{de::DeserializeOwned, Serialize};
+
+use crate::{error, event_modeling::ComponentName};
@@
     self
     }
 }
+
+/// Serve queries for read models as service endpoints.
+///
+/// Implementations are responsible for receiving serialized queries,
+/// dispatching them to the appropriate `QueryHandler`, and returning
+/// serialized results to the caller.
+///
+/// The exact transport mapping is implementation-specific. For example, a
+/// NATS backend derives a request-reply subject from the `ComponentName`
+/// segments: `query.<bounded_context>.<domain>.<feature>.<component>`.
+#[trait_variant::make(Send)]
+pub trait QueryService {
+    /// Start serving queries for the given query specification.
+    ///
+    /// Implementations typically keep running until the underlying transport
+    /// is closed or an unrecoverable error occurs.
+    fn serve<H>(&self, spec: &QuerySpec<H>) -> impl Future<Output = error::Result<()>> + Send
+    where
+        H: QueryHandler + 'static,
+        H::Query: DeserializeOwned,
+        H::Id: DeserializeOwned,
+        <H::Query as Query>::ReadModel: Serialize,
+        <H::Query as Query>::Response: Serialize;
+}
+
+/// Send queries to read model service endpoints.
+///
+/// Implementations are responsible for serializing queries, routing them
+/// to the appropriate transport endpoint derived from the `ComponentName`,
+/// awaiting the service reply, and mapping transport or service failures
+/// back into [`error::Error`].
+///
+/// The subject derivation convention for NATS request-reply is:
+/// `query.<bounded_context>.<domain>.<feature>.<component>`.
+#[trait_variant::make(Send)]
+pub trait QueryClient {
+    /// Fetch a single read model instance by its identifier.
+    ///
+    /// Serializes the `id`, routes it to the service endpoint associated
+    /// with the given `ComponentName`, then waits for the reply.
+    ///
+    /// Returns `Ok(None)` when the read model is not found.
+    async fn get_by_id<Q, Id>(
+        &self,
+        name: &ComponentName,
+        id: Id,
+    ) -> error::Result<Option<Q::ReadModel>>
+    where
+        Q: Query,
+        Q::ReadModel: DeserializeOwned,
+        Id: Serialize + Send;
+
+    /// Send a custom query and await the response.
+    ///
+    /// Serializes the `query`, routes it to the service endpoint associated
+    /// with the given `ComponentName`, then waits for the reply.
+    async fn query<Q>(
+        &self,
+        name: &ComponentName,
+        query: Q,
+    ) -> error::Result<Q::Response>
+    where
+        Q: Query + Serialize,
+        Q::Response: DeserializeOwned;
+}
</FILE_PATCH>

<FILE_PATCH file_path="src/event_modeling.rs">
@@
     }
 
+    /// Returns the query service subject derived from this component name.
+    ///
+    /// The convention is `query.<bounded_context>.<domain>.<feature>.<component>`.
+    pub fn query_subject(&self) -> String {
+        format!(
+            "query.{}.{}.{}.{}",
+            self.bounded_context, self.domain, self.feature, self.component
+        )
+    }
+
     /// Returns the structured slice path without the consumer segment.
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git commit -a -m "+ query - Add QueryService and QueryClient traits with subject derivation helper"
</suggested_git_command>

