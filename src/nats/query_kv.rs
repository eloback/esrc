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
pub type QueryFuture<'a, T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<T>> + Send + 'a>>;

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
        let bytes = serde_json::to_vec(model).map_err(|e| Error::Format(e.into()))?;

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
                let model: RM =
                    serde_json::from_slice(&entry).map_err(|e| Error::Format(e.into()))?;
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

    async fn get_by_id(&self, id: Self::Id) -> error::Result<Option<RM>> {
        self.get(&id).await
    }

    async fn handle(&self, query: Q) -> error::Result<Q::Response> {
        let store_clone = self.clone();
        (self.inner.query_fn)(query, store_clone).await
    }
}
