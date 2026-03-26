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
