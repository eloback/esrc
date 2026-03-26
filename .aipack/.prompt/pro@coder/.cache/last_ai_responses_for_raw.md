====
>   Info: Duration: 1m 4s | ~$0.2952 | Prompt Tokens: 31,608 (cached: 0, cache_creation: 0) | Completion Tokens: 5,488 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.



Looking at the plan files, the active step is "In-memory QueryHandler helper for View-based live projections" which is already implemented. The next step in `plan-1-todo-steps.md` is "NATS KV-backed QueryHandler implementation".

Since the active step is already done, I need to:
1. Move the active step to done
2. Activate the next todo step and implement it

However, looking at the NATS KV step, I need to check if I have the necessary files. Let me review what I have:

- `src/nats.rs` ✓
- `src/nats/query_service.rs` ✓
- `src/query/mod.rs` ✓
- `src/query/in_memory.rs` ✓
- `src/event_modeling.rs` ✓

I have sufficient context to implement the NATS KV-backed QueryHandler. Let me proceed.

<FILE_CHANGES>

<FILE_PATCH file_path="_workbench/queries/plan-3-done-steps.md">
@@
-## Step - Implement NATS QueryService and QueryClient
+## Step - In-memory QueryHandler helper for View-based live projections
       status: done
-time-created: 2026-03-26 14:58:37
-   time-done: 2026-03-26 15:36:55
+time-created: 2026-03-26 15:39:52
+   time-done: 2026-03-26 16:28:37
 
-- Created `src/nats/query_service.rs` with full NATS-backed implementations.
+- Created `src/query/in_memory.rs` with `InMemoryViewStore<RM, Q>`, a thread-safe, in-memory store for read model instances keyed by `Uuid`.
 
-- Implemented `QueryService` for `NatsStore`:
-  - Derives NATS request-reply subject from `ComponentName::query_subject()`: `query.<bounded_context>.<domain>.<feature>.<component>`.
-  - Uses the async-nats service builder to create a service group and endpoint, following the same pattern as `CommandService` in `src/nats/command_service.rs`.
-  - Deserializes incoming `QueryRequest` (either `GetById` or `Query` variant), dispatches to the `QueryHandler`, serializes and sends back results.
-  - Includes serializable reply/error types: `QueryRequest`, `QueryReplyError`, `GetByIdReply`, `QueryReply`.
-  - Error handling follows the same pattern as `CommandReply`/`ReplyError`, with fallback error replies if serialization fails.
+- `InMemoryViewStore` provides:
+  - `new(query_fn)`: constructor with closure for custom query logic.
+  - `upsert`, `remove`, `get`, `all`, `len`, `is_empty` methods.
 
-- Implemented `QueryClient` for `NatsStore`:
-  - `get_by_id`: serializes a `QueryRequest::GetById`, sends as NATS request to `<query_subject>.query`, deserializes `GetByIdReply`.
-  - `query`: serializes a `QueryRequest::Query`, sends as NATS request, deserializes `QueryReply`.
-  - Maps transport/service errors back to `esrc::error::Error::Internal`.
+- Implements `QueryHandler` with `type Id = Uuid`, delegating to the store and closure.
 
-- Added `spawn_query_service` method to `NatsStore`:
-  - Follows the `spawn_service` pattern with `Tripwire`-based graceful shutdown.
-  - Registers a shutdown trigger, wraps `serve` in a tracked cancellable task.
+- Converted `src/query.rs` to `src/query/mod.rs` module directory.
 
-- Registered `pub mod query_service` in `src/nats.rs`.
+- Store is cheaply cloneable (`Arc`-wrapped), shareable between `Project` (write) and `QuerySpec` (read).
+
+## Step - Implement NATS QueryService and QueryClient
+      status: done
+time-created: 2026-03-26 14:58:37
+   time-done: 2026-03-26 15:36:55
+
+- Created `src/nats/query_service.rs` with full NATS-backed implementations.
+
+- Implemented `QueryService` for `NatsStore`:
+  - Derives NATS request-reply subject from `ComponentName::query_subject()`: `query.<bounded_context>.<domain>.<feature>.<component>`.
+  - Uses the async-nats service builder to create a service group and endpoint, following the same pattern as `CommandService` in `src/nats/command_service.rs`.
+  - Deserializes incoming `QueryRequest` (either `GetById` or `Query` variant), dispatches to the `QueryHandler`, serializes and sends back results.
+  - Includes serializable reply/error types: `QueryRequest`, `QueryReplyError`, `GetByIdReply`, `QueryReply`.
+  - Error handling follows the same pattern as `CommandReply`/`ReplyError`, with fallback error replies if serialization fails.
+
+- Implemented `QueryClient` for `NatsStore`:
+  - `get_by_id`: serializes a `QueryRequest::GetById`, sends as NATS request to `<query_subject>.query`, deserializes `GetByIdReply`.
+  - `query`: serializes a `QueryRequest::Query`, sends as NATS request, deserializes `QueryReply`.
+  - Maps transport/service errors back to `esrc::error::Error::Internal`.
+
+- Added `spawn_query_service` method to `NatsStore`:
+  - Follows the `spawn_service` pattern with `Tripwire`-based graceful shutdown.
+  - Registers a shutdown trigger, wraps `serve` in a tracked cancellable task.
+
+- Registered `pub mod query_service` in `src/nats.rs`.
</FILE_PATCH>

<FILE_NEW file_path="_workbench/queries/plan-2-active-step.md">
# Plan 2 - Active Step

## Step - NATS KV-backed QueryHandler implementation
      status: active
time-created: 2026-03-26 15:39:52
time-current: 2026-03-26 16:28:37

- Create a concrete `QueryHandler` implementation backed by NATS JetStream Key-Value store.
- This provides a persistence layer for read models without requiring the developer to set up external storage.
- Design considerations:
  - The KV bucket name should be derived from the `ComponentName` (convention over configuration), with an option to override.
  - `get_by_id` reads from the KV bucket by key.
  - The `Project` implementation writes to the same KV bucket (the developer calls a save/put method inside their `project()` implementation).
  - Provide a helper or wrapper that the developer can hold inside their `Project` impl to write to the KV store, and that also implements `QueryHandler` for reads.
  - Serialization format: JSON (consistent with other NATS message payloads in the crate).
- Module placement: `src/nats/query_kv.rs` or similar, registered in `src/nats.rs`.
- References: see `src/query.rs` for `QueryHandler`, `src/nats.rs` for `NatsStore`.

</FILE_NEW>

<FILE_PATCH file_path="_workbench/queries/plan-1-todo-steps.md">
@@
 # Plan 1 - Todo Steps
 
-## Step - NATS KV-backed QueryHandler implementation
-      status: not_started
-time-created: 2026-03-26 15:39:52
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
-
 ## Step - Vertical slice helper composing ConsumerSpec and QuerySpec
</FILE_PATCH>

<FILE_NEW file_path="src/nats/query_kv.rs">
//! NATS JetStream Key-Value backed [`QueryHandler`] implementation.
//!
//! Provides [`NatsKvStore`], a shared store that persists read model instances
//! in a NATS JetStream Key-Value bucket. It can be used both as the write
//! target inside a [`Project`](crate::project::Project) implementation and as
//! the read source for a [`QueryHandler`].
//!
//! # Usage
//!
//! 1. Create an `NatsKvStore` from a `NatsStore` (or directly from a JetStream
//!    context) with a query handler closure.
//! 2. Clone the store into your `Project` implementation and call
//!    [`put`](NatsKvStore::put) / [`delete`](NatsKvStore::delete) inside
//!    `project()`.
//! 3. Pass the same store (or a clone) as the `QueryHandler` to a `QuerySpec`.
//!
//! # Example
//!
//! ```rust,ignore
//! use esrc::nats::query_kv::NatsKvStore;
//!
//! let kv_store: NatsKvStore<MyReadModel, MyQuery> = NatsKvStore::new(
//!     &nats_store,
//!     &component_name,
//!     |query, getter| Box::pin(async move {
//!         match query {
//!             MyQuery::ListActive => {
//!                 // For list queries, the developer must implement their own
//!                 // iteration strategy or maintain secondary indices.
//!                 todo!("implement custom query logic")
//!             }
//!         }
//!     }),
//! ).await?;
//!
//! // Inside your Project impl:
//! // kv_store.put("some-id", &read_model).await?;
//!
//! // Wire as QueryHandler via QuerySpec:
//! // QuerySpec::new(name, transport, kv_store.clone())
//! ```

use std::sync::Arc;

use async_nats::jetstream::kv::{Config as KvConfig, Store as KvBucket};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{self, Error};
use crate::event_modeling::ComponentName;
use crate::query::{Query, QueryHandler};

/// A type alias for the boxed future returned by the query function closure.
pub type QueryFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<T>> + Send + 'a>>;

/// A read/write store for read model instances backed by a NATS JetStream
/// Key-Value bucket.
///
/// This type is cheaply cloneable (internally `Arc`-wrapped) and can be shared
/// between a `Project` implementation (write side) and a `QueryHandler` (read
/// side).
///
/// The generic parameter `Q` is the user's `Query` enum type, which determines
/// the `ReadModel` and `Response` associated types. Custom query logic is
/// supplied via a closure at construction time. The closure receives the query
/// enum and a reference to the `NatsKvStore` itself (for key lookups), and
/// returns a future producing the query response.
pub struct NatsKvStore<RM, Q>
where
    RM: Clone + Send + Sync,
    Q: Query<ReadModel = RM>,
{
    inner: Arc<Inner<RM, Q>>,
}

struct Inner<RM, Q>
where
    RM: Clone + Send + Sync,
    Q: Query<ReadModel = RM>,
{
    bucket: KvBucket,
    query_fn: Box<dyn Fn(Q, NatsKvStore<RM, Q>) -> QueryFuture<'static, Q::Response> + Send + Sync>,
}

impl<RM, Q> Clone for NatsKvStore<RM, Q>
where
    RM: Clone + Send + Sync,
    Q: Query<ReadModel = RM>,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<RM, Q> std::fmt::Debug for NatsKvStore<RM, Q>
where
    RM: Clone + Send + Sync,
    Q: Query<ReadModel = RM>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NatsKvStore")
            .field("bucket", &self.inner.bucket.status().map(|_| "connected"))
            .finish()
    }
}

impl<RM, Q> NatsKvStore<RM, Q>
where
    RM: Clone + Send + Sync + Serialize + DeserializeOwned + 'static,
    Q: Query<ReadModel = RM> + Send + 'static,
    Q::Response: Send + 'static,
{
    /// Create a new NATS KV store with the bucket name derived from the
    /// `ComponentName`.
    ///
    /// The bucket name is derived using `ComponentName::durable_name()`,
    /// prefixed with `rm_` (read model). The bucket is created if it does not
    /// already exist.
    ///
    /// The `query_fn` closure receives the user's query enum value and a clone
    /// of this `NatsKvStore` (so the closure can call `get`, `get_by_id`, etc.
    /// for lookups). It returns a boxed future producing the query response.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let store = NatsKvStore::new(&nats_store, &name, |query, store| {
    ///     Box::pin(async move {
    ///         match query {
    ///             MyQuery::ListActive => todo!(),
    ///         }
    ///     })
    /// }).await?;
    /// ```
    pub async fn new<F>(
        nats_store: &super::NatsStore,
        name: &ComponentName,
        query_fn: F,
    ) -> error::Result<Self>
    where
        F: Fn(Q, NatsKvStore<RM, Q>) -> QueryFuture<'static, Q::Response> + Send + Sync + 'static,
    {
        let bucket_name = format!("rm_{}", name.durable_name());
        Self::with_bucket_name(nats_store, &bucket_name, query_fn).await
    }

    /// Create a new NATS KV store with an explicit bucket name.
    ///
    /// Use this when the default derived bucket name is not suitable.
    pub async fn with_bucket_name<F>(
        nats_store: &super::NatsStore,
        bucket_name: &str,
        query_fn: F,
    ) -> error::Result<Self>
    where
        F: Fn(Q, NatsKvStore<RM, Q>) -> QueryFuture<'static, Q::Response> + Send + Sync + 'static,
    {
        let context = nats_store.jetstream_context();

        let bucket = context
            .create_key_value(KvConfig {
                bucket: bucket_name.to_owned(),
                ..Default::default()
            })
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        Ok(Self {
            inner: Arc::new(Inner {
                bucket,
                query_fn: Box::new(query_fn),
            }),
        })
    }

    /// Create a new NATS KV store directly from a JetStream context and
    /// bucket name.
    ///
    /// This is useful when a `NatsStore` instance is not available.
    pub async fn from_context<F>(
        context: &async_nats::jetstream::Context,
        bucket_name: &str,
        query_fn: F,
    ) -> error::Result<Self>
    where
        F: Fn(Q, NatsKvStore<RM, Q>) -> QueryFuture<'static, Q::Response> + Send + Sync + 'static,
    {
        let bucket = context
            .create_key_value(KvConfig {
                bucket: bucket_name.to_owned(),
                ..Default::default()
            })
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        Ok(Self {
            inner: Arc::new(Inner {
                bucket,
                query_fn: Box::new(query_fn),
            }),
        })
    }

    /// Put (insert or update) a read model instance by its string key.
    ///
    /// The read model is serialized to JSON before storage.
    pub async fn put(&self, key: &str, model: &RM) -> error::Result<()> {
        let bytes = serde_json::to_vec(model)
            .map_err(|e| Error::Format(e.into()))?;

        self.inner
            .bucket
            .put(key, bytes.into())
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        Ok(())
    }

    /// Delete a read model instance by its string key.
    ///
    /// This is a soft delete in NATS KV (places a delete marker).
    pub async fn delete(&self, key: &str) -> error::Result<()> {
        self.inner
            .bucket
            .delete(key)
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        Ok(())
    }

    /// Get a read model instance by its string key.
    ///
    /// Returns `None` if the key does not exist or has been deleted.
    pub async fn get(&self, key: &str) -> error::Result<Option<RM>> {
        match self.inner.bucket.get(key).await {
            Ok(Some(entry)) => {
                let model: RM = serde_json::from_slice(&entry)
                    .map_err(|e| Error::Format(e.into()))?;
                Ok(Some(model))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(Error::Internal(e.into())),
        }
    }

    /// Returns a reference to the underlying NATS KV bucket.
    ///
    /// This can be used for advanced operations not covered by the
    /// convenience methods (e.g., watching for changes, listing keys).
    pub fn bucket(&self) -> &KvBucket {
        &self.inner.bucket
    }
}

impl<RM, Q> QueryHandler for NatsKvStore<RM, Q>
where
    RM: Clone + Send + Sync + Serialize + DeserializeOwned + 'static,
    Q: Query<ReadModel = RM> + Send + 'static,
    Q::Response: Send + 'static,
{
    type Query = Q;
    type Id = String;

    async fn get_by_id(
        &self,
        id: Self::Id,
    ) -> error::Result<Option<RM>> {
        self.get(&id).await
    }

    async fn handle(
        &self,
        query: Q,
    ) -> error::Result<Q::Response> {
        let store_clone = self.clone();
        (self.inner.query_fn)(query, store_clone).await
    }
}
</FILE_NEW>

<FILE_PATCH file_path="src/nats.rs">
@@
 /// NATS-backed command service support for `NatsStore`.
 pub mod command_service;
 #[doc(hidden)]
 pub mod convert;
 /// Use a Jetstream message as an esrc Envelope.
 pub mod envelope;
 #[doc(hidden)]
 pub mod event;
 /// NATS-backed query service support for `NatsStore`.
 pub mod query_service;
+/// NATS JetStream Key-Value backed QueryHandler implementation.
+pub mod query_kv;
@@
     pub fn client(&self) -> async_nats::Client {
         self.context.client()
     }
 
+    /// Returns the underlying JetStream context.
+    ///
+    /// This can be used to create Key-Value stores or perform other
+    /// JetStream operations outside the event store abstraction.
+    pub fn jetstream_context(&self) -> &Context {
+        &self.context
+    }
+
     /// Select the stream used for creating read-side consumers.
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git add -A . && git commit -a -m "+ nats/query_kv - Add NATS KV-backed QueryHandler implementation"
</suggested_git_command>

