//! In-memory query handler helper for `View`-based live projections.
//!
//! Provides [`InMemoryViewStore`], a shared, thread-safe store that can be
//! used both as the write target inside a [`Project`](crate::project::Project)
//! implementation and as the read source for a [`QueryHandler`].
//!
//! # Usage
//!
//! 1. Create an `InMemoryViewStore` with a query handler closure.
//! 2. Clone the store into your `Project` implementation and call
//!    [`upsert`](InMemoryViewStore::upsert) /
//!    [`remove`](InMemoryViewStore::remove) inside `project()`.
//! 3. Pass the same store (or a clone) as the `QueryHandler` to a `QuerySpec`.
//!
//! # Example
//!
//! ```rust,ignore
//! use esrc::query::in_memory::InMemoryViewStore;
//!
//! let store: InMemoryViewStore<MyReadModel, MyQuery> = InMemoryViewStore::new(|query, snapshot| {
//!     match query {
//!         MyQuery::ListAll => Ok(snapshot.values().cloned().collect()),
//!         MyQuery::ListActive => Ok(snapshot.values().filter(|m| m.active).cloned().collect()),
//!     }
//! });
//!
//! // Inside your Project impl:
//! // store.upsert(id, read_model);
//!
//! // Wire as QueryHandler via QuerySpec:
//! // QuerySpec::new(name, transport, store.clone())
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use uuid::Uuid;

use super::{Query, QueryHandler};
use crate::error;

/// A thread-safe, in-memory store for read model instances keyed by `Uuid`.
///
/// This type is cheaply cloneable (internally `Arc`-wrapped) and can be shared
/// between a `Project` implementation (write side) and a `QueryHandler` (read
/// side).
///
/// The generic parameter `Q` is the user's `Query` enum type, which determines
/// the `ReadModel` and `Response` associated types. Custom query logic is
/// supplied via a closure at construction time.
pub struct InMemoryViewStore<RM, Q>
where
    RM: Clone + Send + Sync,
    Q: Query<ReadModel = RM>,
{
    inner: Arc<RwLock<HashMap<Uuid, RM>>>,
    query_fn: Arc<dyn Fn(Q, &HashMap<Uuid, RM>) -> error::Result<Q::Response> + Send + Sync>,
}

impl<RM, Q> Clone for InMemoryViewStore<RM, Q>
where
    RM: Clone + Send + Sync,
    Q: Query<ReadModel = RM>,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            query_fn: Arc::clone(&self.query_fn),
        }
    }
}

impl<RM, Q> std::fmt::Debug for InMemoryViewStore<RM, Q>
where
    RM: Clone + Send + Sync,
    Q: Query<ReadModel = RM>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryViewStore")
            .field("entries", &self.inner.read().map(|g| g.len()).unwrap_or(0))
            .finish()
    }
}

impl<RM, Q> InMemoryViewStore<RM, Q>
where
    RM: Clone + Send + Sync + 'static,
    Q: Query<ReadModel = RM>,
{
    /// Create a new in-memory view store with a custom query handler closure.
    ///
    /// The `query_fn` receives the user's query enum value and an immutable
    /// reference to the full snapshot (`HashMap<Uuid, RM>`) of the current
    /// store contents. It should return the query response or an error.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let store = InMemoryViewStore::new(|query, snapshot| {
    ///     match query {
    ///         MyQuery::ListAll => Ok(snapshot.values().cloned().collect()),
    ///     }
    /// });
    /// ```
    pub fn new<F>(query_fn: F) -> Self
    where
        F: Fn(Q, &HashMap<Uuid, RM>) -> error::Result<Q::Response> + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            query_fn: Arc::new(query_fn),
        }
    }

    /// Insert or update a read model instance by its identifier.
    pub fn upsert(&self, id: Uuid, model: RM) {
        self.inner
            .write()
            .expect("lock not poisoned")
            .insert(id, model);
    }

    /// Remove a read model instance by its identifier.
    ///
    /// Returns the removed instance if it existed.
    pub fn remove(&self, id: &Uuid) -> Option<RM> {
        self.inner
            .write()
            .expect("lock not poisoned")
            .remove(id)
    }

    /// Get a clone of a read model instance by its identifier.
    pub fn get(&self, id: &Uuid) -> Option<RM> {
        self.inner
            .read()
            .expect("lock not poisoned")
            .get(id)
            .cloned()
    }

    /// Get a snapshot of all entries as a `Vec`.
    pub fn all(&self) -> Vec<RM> {
        self.inner
            .read()
            .expect("lock not poisoned")
            .values()
            .cloned()
            .collect()
    }

    /// Returns the number of entries currently in the store.
    pub fn len(&self) -> usize {
        self.inner.read().expect("lock not poisoned").len()
    }

    /// Returns `true` if the store contains no entries.
    pub fn is_empty(&self) -> bool {
        self.inner.read().expect("lock not poisoned").is_empty()
    }
}

impl<RM, Q> QueryHandler for InMemoryViewStore<RM, Q>
where
    RM: Clone + Send + Sync + 'static,
    Q: Query<ReadModel = RM> + Send + 'static,
    Q::Response: Send + 'static,
{
    type Query = Q;
    type Id = Uuid;

    async fn get_by_id(
        &self,
        id: Self::Id,
    ) -> error::Result<Option<RM>> {
        Ok(self.get(&id))
    }

    async fn handle(
        &self,
        query: Q,
    ) -> error::Result<Q::Response> {
        let guard = self.inner.read().expect("lock not poisoned");
        (self.query_fn)(query, &guard)
    }
}
