====
>   Info: Duration: 1m 49s | ~$0.5285 | Prompt Tokens: 58,057 (cached: 0, cache_creation: 0) | Completion Tokens: 9,527 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 42 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.



Looking at the plan files, the current active step is empty and there's no step currently being implemented. The next step in `plan-1-todo-steps.md` is "In-memory QueryHandler helper for View-based live projections".

Let me review what's needed and implement it.

The step requires creating a simple utility that materializes a `View` from an event stream and serves queries against live in-memory state. However, looking more carefully at the step description and the existing code, I need to think about what a practical in-memory QueryHandler helper looks like.

The `View` trait is `Default + Send` with an `apply` method, but it has no notion of identity or storage. The helper needs to:
1. Hold view state internally (behind synchronization)
2. Implement `QueryHandler` (get_by_id + handle)
3. Be a "simpler utility with good UX" per the user's direction

Since `View` operates on a single event stream per aggregate ID, and the `QueryHandler` needs `get_by_id` by some ID, a natural approach is a `HashMap<Id, V>` where `Id` is the aggregate UUID. The user needs to provide logic for how events map to view state updates.

Looking at the existing patterns and the `Project` trait, I think the best approach is:
- An `InMemoryViewStore<V>` that holds `Arc<RwLock<HashMap<Uuid, V>>>` and implements `QueryHandler`
- The user uses this store inside their `Project` implementation to write view state
- The same store serves as the `QueryHandler` for reads

This matches the pattern already demonstrated in the `basic-query-service` example with `OrderStore`, but generalized.

<FILE_CHANGES>

<FILE_NEW file_path="src/query/mod.rs">
//! Traits and types for declaring and handling queries against read models.

use std::future::Future;

use serde::{de::DeserializeOwned, Serialize};

use crate::{error, event_modeling::ComponentName};

/// In-memory query handler helper for View-based live projections.
pub mod in_memory;

/// A query that can be executed against a read model.
///
/// Defines the association between a query enum, its target read model,
/// and the response type. Kept minimal with no serde bounds; serialization
/// requirements are pushed to transport-specific traits.
pub trait Query: Send {
    /// The read model this query targets.
    type ReadModel: Send;
    /// The response type returned by executing this query.
    type Response: Send;
}

/// Handles queries for a specific read model.
///
/// Includes a built-in `get_by_id` method that every handler must implement,
/// plus a `handle` method for custom query enum variants. Uses `esrc::error::Error`
/// with `External` for domain/persistence errors (matching the command handler model).
///
/// Does not require `Clone`; sharing via `Arc` is expected.
#[trait_variant::make(Send)]
pub trait QueryHandler: Send + Sync {
    /// The query enum this handler responds to.
    type Query: Query;
    /// The identifier type used to look up individual read model instances.
    type Id: Send + Sync;

    /// Get a single read model instance by its identifier.
    ///
    /// Returns `Ok(None)` when the read model is not found.
    async fn get_by_id(
        &self,
        id: Self::Id,
    ) -> crate::error::Result<Option<<Self::Query as Query>::ReadModel>>;

    /// Execute a custom query.
    async fn handle(
        &self,
        query: Self::Query,
    ) -> crate::error::Result<<Self::Query as Query>::Response>;
}

/// Transport mechanism for exposing queries remotely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryTransport {
    /// Expose queries via NATS request-reply.
    NatsRequestReply,
}

/// A normalized query declaration that can later be executed by infrastructure.
///
/// Analogous to `ConsumerSpec`, declares metadata about how queries are exposed.
#[derive(Clone, Debug)]
pub struct QuerySpec<H> {
    name: ComponentName,
    transport: QueryTransport,
    handler: H,
}

impl<H> QuerySpec<H> {
    /// Create a new query specification.
    pub fn new(name: ComponentName, transport: QueryTransport, handler: H) -> Self {
        Self {
            name,
            transport,
            handler,
        }
    }

    /// Returns the structured component name for this query.
    pub fn name(&self) -> &ComponentName {
        &self.name
    }

    /// Returns the configured transport for this query.
    pub fn transport(&self) -> QueryTransport {
        self.transport
    }

    /// Returns a reference to the configured query handler.
    pub fn handler(&self) -> &H {
        &self.handler
    }

    /// Returns a mutable reference to the configured query handler.
    pub fn handler_mut(&mut self) -> &mut H {
        &mut self.handler
    }

    /// Consumes the specification and returns the configured query handler.
    pub fn into_handler(self) -> H {
        self.handler
    }

    /// Override the transport for this query.
    pub fn with_transport(mut self, transport: QueryTransport) -> Self {
        self.transport = transport;
        self
    }
}

/// Serve queries for read models as service endpoints.
///
/// Implementations are responsible for receiving serialized queries,
/// dispatching them to the appropriate `QueryHandler`, and returning
/// serialized results to the caller.
///
/// The exact transport mapping is implementation-specific. For example, a
/// NATS backend derives a request-reply subject from the `ComponentName`
/// segments: `query.<bounded_context>.<domain>.<feature>.<component>`.
#[trait_variant::make(Send)]
pub trait QueryService {
    /// Start serving queries for the given query specification.
    ///
    /// Implementations typically keep running until the underlying transport
    /// is closed or an unrecoverable error occurs.
    fn serve<H>(&self, spec: &QuerySpec<H>) -> impl Future<Output = error::Result<()>> + Send
    where
        H: QueryHandler + Send + Sync + 'static,
        H::Query: DeserializeOwned + Sync,
        H::Id: DeserializeOwned,
        <H::Query as Query>::ReadModel: Serialize + Sync,
        <H::Query as Query>::Response: Serialize + Sync;
}

/// Send queries to read model service endpoints.
///
/// Implementations are responsible for serializing queries, routing them
/// to the appropriate transport endpoint derived from the `ComponentName`,
/// awaiting the service reply, and mapping transport or service failures
/// back into [`error::Error`].
///
/// The subject derivation convention for NATS request-reply is:
/// `query.<bounded_context>.<domain>.<feature>.<component>`.
#[trait_variant::make(Send)]
pub trait QueryClient {
    /// Fetch a single read model instance by its identifier.
    ///
    /// Serializes the `id`, routes it to the service endpoint associated
    /// with the given `ComponentName`, then waits for the reply.
    ///
    /// Returns `Ok(None)` when the read model is not found.
    async fn get_by_id<Q, Id>(
        &self,
        name: &ComponentName,
        id: Id,
    ) -> error::Result<Option<Q::ReadModel>>
    where
        Q: Query,
        Q::ReadModel: DeserializeOwned,
        Id: Serialize + Send;

    /// Send a custom query and await the response.
    ///
    /// Serializes the `query`, routes it to the service endpoint associated
    /// with the given `ComponentName`, then waits for the reply.
    async fn query<Q>(&self, name: &ComponentName, query: Q) -> error::Result<Q::Response>
    where
        Q: Query + Serialize,
        Q::Response: DeserializeOwned;
}
</FILE_NEW>

<FILE_NEW file_path="src/query/in_memory.rs">
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
</FILE_NEW>

<FILE_PATCH file_path="src/lib.rs">
@@
-/// Traits and types for declaring and handling queries against read models.
-pub mod query;
+/// Traits and types for declaring and handling queries against read models.
+pub mod query;
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/queries/plan-1-todo-steps.md">
@@
 # Plan 1 - Todo Steps
 
-## Step - In-memory QueryHandler helper for View-based live projections
-      status: not_started
-time-created: 2026-03-26 15:39:52
-
-- Create a simple utility that materializes a `View` from an event stream (replay + subscribe) and serves queries against the live in-memory state.
-- This should be a concrete `QueryHandler` implementation that:
-  - Holds a `View` instance internally (behind appropriate synchronization).
-  - Implements `get_by_id` and `handle` against the in-memory state.
-- Design considerations:
-  - The user mentioned "simpler utility with good UX", not a full managed runtime component.
-  - The `View` trait is `Default + Send` and has an `apply` method. The helper needs to replay events to build initial state and then subscribe for updates.
-  - The helper should be generic over the `View` type and the storage mechanism (e.g., a `HashMap<Id, V>` keyed by some ID extractor).
-  - Consider how the user provides the ID extraction logic (e.g., a closure or trait method that maps an event to an aggregate ID).
-- Module placement: likely `src/query/in_memory.rs` or a new section in `src/query.rs` depending on size. The user said they would split later, so starting in `src/query.rs` or a submodule is fine.
-- References: see `src/view.rs` for the `View` trait, and `src/query.rs` for `QueryHandler`.
-
 ## Step - NATS KV-backed QueryHandler implementation
</FILE_PATCH>

<FILE_NEW file_path="_workbench/queries/plan-2-active-step.md">
# Plan 2 - Active Step

## Step - In-memory QueryHandler helper for View-based live projections
      status: active
time-created: 2026-03-26 15:39:52
time-current: 2026-03-26 16:23:08

- Created `src/query/in_memory.rs` with `InMemoryViewStore<RM, Q>`, a thread-safe, in-memory store for read model instances keyed by `Uuid`.

- `InMemoryViewStore` provides:
  - `new(query_fn)`: constructor that takes a closure for custom query logic. The closure receives the query enum and an immutable `&HashMap<Uuid, RM>` snapshot.
  - `upsert(id, model)`: insert or update a read model instance.
  - `remove(id)`: remove a read model instance.
  - `get(id)`: get a cloned read model by ID.
  - `all()`: get all entries as a `Vec`.
  - `len()` / `is_empty()`: size introspection.

- Implements `QueryHandler` for `InMemoryViewStore<RM, Q>`:
  - `type Query = Q`, `type Id = Uuid`
  - `get_by_id` delegates to `self.get(&id)`.
  - `handle` acquires a read lock and calls the user-supplied `query_fn` closure.

- Converted `src/query.rs` to `src/query/mod.rs` (module directory) to accommodate the new `in_memory` submodule. All existing content from `src/query.rs` is preserved verbatim in `src/query/mod.rs`.

- Design decisions:
  - The store is cheaply cloneable (`Arc`-wrapped internals), so it can be shared between a `Project` impl (write side) and a `QuerySpec` (read side).
  - No coupling to `View` trait directly; the store is a generic read model container. The user materializes their `View` (or any read model) and calls `upsert` in their `Project::project` method.
  - Custom query logic is injected via a closure rather than requiring the user to implement a separate trait, keeping the UX simple.
  - The `query_fn` closure pattern avoids the need for the user to create a separate handler struct for simple cases, while still allowing full flexibility.
</FILE_NEW>

</FILE_CHANGES>

Wait, I need to verify that converting `src/query.rs` to `src/query/mod.rs` works correctly. The `src/lib.rs` already has `pub mod query;` which will resolve to either `src/query.rs` or `src/query/mod.rs`. Since I'm creating `src/query/mod.rs` as a new file and the old `src/query.rs` needs to be removed, let me adjust.

<FILE_CHANGES>

<FILE_DELETE file_path="src/query.rs" />

<FILE_NEW file_path="src/query/mod.rs">
//! Traits and types for declaring and handling queries against read models.

use std::future::Future;

use serde::{de::DeserializeOwned, Serialize};

use crate::{error, event_modeling::ComponentName};

/// In-memory query handler helper for View-based live projections.
pub mod in_memory;

/// A query that can be executed against a read model.
///
/// Defines the association between a query enum, its target read model,
/// and the response type. Kept minimal with no serde bounds; serialization
/// requirements are pushed to transport-specific traits.
pub trait Query: Send {
    /// The read model this query targets.
    type ReadModel: Send;
    /// The response type returned by executing this query.
    type Response: Send;
}

/// Handles queries for a specific read model.
///
/// Includes a built-in `get_by_id` method that every handler must implement,
/// plus a `handle` method for custom query enum variants. Uses `esrc::error::Error`
/// with `External` for domain/persistence errors (matching the command handler model).
///
/// Does not require `Clone`; sharing via `Arc` is expected.
#[trait_variant::make(Send)]
pub trait QueryHandler: Send + Sync {
    /// The query enum this handler responds to.
    type Query: Query;
    /// The identifier type used to look up individual read model instances.
    type Id: Send + Sync;

    /// Get a single read model instance by its identifier.
    ///
    /// Returns `Ok(None)` when the read model is not found.
    async fn get_by_id(
        &self,
        id: Self::Id,
    ) -> crate::error::Result<Option<<Self::Query as Query>::ReadModel>>;

    /// Execute a custom query.
    async fn handle(
        &self,
        query: Self::Query,
    ) -> crate::error::Result<<Self::Query as Query>::Response>;
}

/// Transport mechanism for exposing queries remotely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryTransport {
    /// Expose queries via NATS request-reply.
    NatsRequestReply,
}

/// A normalized query declaration that can later be executed by infrastructure.
///
/// Analogous to `ConsumerSpec`, declares metadata about how queries are exposed.
#[derive(Clone, Debug)]
pub struct QuerySpec<H> {
    name: ComponentName,
    transport: QueryTransport,
    handler: H,
}

impl<H> QuerySpec<H> {
    /// Create a new query specification.
    pub fn new(name: ComponentName, transport: QueryTransport, handler: H) -> Self {
        Self {
            name,
            transport,
            handler,
        }
    }

    /// Returns the structured component name for this query.
    pub fn name(&self) -> &ComponentName {
        &self.name
    }

    /// Returns the configured transport for this query.
    pub fn transport(&self) -> QueryTransport {
        self.transport
    }

    /// Returns a reference to the configured query handler.
    pub fn handler(&self) -> &H {
        &self.handler
    }

    /// Returns a mutable reference to the configured query handler.
    pub fn handler_mut(&mut self) -> &mut H {
        &mut self.handler
    }

    /// Consumes the specification and returns the configured query handler.
    pub fn into_handler(self) -> H {
        self.handler
    }

    /// Override the transport for this query.
    pub fn with_transport(mut self, transport: QueryTransport) -> Self {
        self.transport = transport;
        self
    }
}

/// Serve queries for read models as service endpoints.
///
/// Implementations are responsible for receiving serialized queries,
/// dispatching them to the appropriate `QueryHandler`, and returning
/// serialized results to the caller.
///
/// The exact transport mapping is implementation-specific. For example, a
/// NATS backend derives a request-reply subject from the `ComponentName`
/// segments: `query.<bounded_context>.<domain>.<feature>.<component>`.
#[trait_variant::make(Send)]
pub trait QueryService {
    /// Start serving queries for the given query specification.
    ///
    /// Implementations typically keep running until the underlying transport
    /// is closed or an unrecoverable error occurs.
    fn serve<H>(&self, spec: &QuerySpec<H>) -> impl Future<Output = error::Result<()>> + Send
    where
        H: QueryHandler + Send + Sync + 'static,
        H::Query: DeserializeOwned + Sync,
        H::Id: DeserializeOwned,
        <H::Query as Query>::ReadModel: Serialize + Sync,
        <H::Query as Query>::Response: Serialize + Sync;
}

/// Send queries to read model service endpoints.
///
/// Implementations are responsible for serializing queries, routing them
/// to the appropriate transport endpoint derived from the `ComponentName`,
/// awaiting the service reply, and mapping transport or service failures
/// back into [`error::Error`].
///
/// The subject derivation convention for NATS request-reply is:
/// `query.<bounded_context>.<domain>.<feature>.<component>`.
#[trait_variant::make(Send)]
pub trait QueryClient {
    /// Fetch a single read model instance by its identifier.
    ///
    /// Serializes the `id`, routes it to the service endpoint associated
    /// with the given `ComponentName`, then waits for the reply.
    ///
    /// Returns `Ok(None)` when the read model is not found.
    async fn get_by_id<Q, Id>(
        &self,
        name: &ComponentName,
        id: Id,
    ) -> error::Result<Option<Q::ReadModel>>
    where
        Q: Query,
        Q::ReadModel: DeserializeOwned,
        Id: Serialize + Send;

    /// Send a custom query and await the response.
    ///
    /// Serializes the `query`, routes it to the service endpoint associated
    /// with the given `ComponentName`, then waits for the reply.
    async fn query<Q>(&self, name: &ComponentName, query: Q) -> error::Result<Q::Response>
    where
        Q: Query + Serialize,
        Q::Response: DeserializeOwned;
}
</FILE_NEW>

<FILE_NEW file_path="src/query/in_memory.rs">
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
</FILE_NEW>

<FILE_PATCH file_path="_workbench/queries/plan-1-todo-steps.md">
@@
 # Plan 1 - Todo Steps
 
-## Step - In-memory QueryHandler helper for View-based live projections
-      status: not_started
-time-created: 2026-03-26 15:39:52
-
-- Create a simple utility that materializes a `View` from an event stream (replay + subscribe) and serves queries against the live in-memory state.
-- This should be a concrete `QueryHandler` implementation that:
-  - Holds a `View` instance internally (behind appropriate synchronization).
-  - Implements `get_by_id` and `handle` against the in-memory state.
-- Design considerations:
-  - The user mentioned "simpler utility with good UX", not a full managed runtime component.
-  - The `View` trait is `Default + Send` and has an `apply` method. The helper needs to replay events to build initial state and then subscribe for updates.
-  - The helper should be generic over the `View` type and the storage mechanism (e.g., a `HashMap<Id, V>` keyed by some ID extractor).
-  - Consider how the user provides the ID extraction logic (e.g., a closure or trait method that maps an event to an aggregate ID).
-- Module placement: likely `src/query/in_memory.rs` or a new section in `src/query.rs` depending on size. The user said they would split later, so starting in `src/query.rs` or a submodule is fine.
-- References: see `src/view.rs` for the `View` trait, and `src/query.rs` for `QueryHandler`.
-
 ## Step - NATS KV-backed QueryHandler implementation
</FILE_PATCH>

<FILE_NEW file_path="_workbench/queries/plan-2-active-step.md">
# Plan 2 - Active Step

## Step - In-memory QueryHandler helper for View-based live projections
      status: active
time-created: 2026-03-26 15:39:52
time-current: 2026-03-26 16:23:08

- Created `src/query/in_memory.rs` with `InMemoryViewStore<RM, Q>`, a thread-safe, in-memory store for read model instances keyed by `Uuid`.

- Converted `src/query.rs` into a module directory `src/query/mod.rs` to accommodate the new submodule. All existing content preserved verbatim.

- `InMemoryViewStore` provides:
  - `new(query_fn)`: constructor that takes a closure for custom query logic. The closure receives the query enum and an immutable `&HashMap<Uuid, RM>` snapshot.
  - `upsert(id, model)`: insert or update a read model instance.
  - `remove(id)`: remove a read model instance, returning it if it existed.
  - `get(id)`: get a cloned read model by ID.
  - `all()`: get all entries as a `Vec`.
  - `len()` / `is_empty()`: size introspection.

- Implements `QueryHandler` for `InMemoryViewStore<RM, Q>`:
  - `type Query = Q`, `type Id = Uuid`
  - `get_by_id` delegates to `self.get(&id)`.
  - `handle` acquires a read lock and calls the user-supplied `query_fn` closure.

- Design decisions:
  - The store is cheaply cloneable (`Arc`-wrapped internals), so it can be shared between a `Project` impl (write side) and a `QuerySpec` (read side).
  - No coupling to `View` trait directly; the store is a generic read model container. The user materializes their `View` (or any read model) and calls `upsert` in their `Project::project` method.
  - Custom query logic is injected via a closure rather than requiring the user to implement a separate trait, keeping the UX simple.
  - The `query_fn` closure pattern avoids the need for the user to create a separate handler struct for simple cases, while still allowing full flexibility.
</FILE_NEW>

</FILE_CHANGES>

<suggested_git_command>
git add -A . && git commit -m "+ query/in_memory - Add InMemoryViewStore as generic in-memory QueryHandler helper"
</suggested_git_command>

