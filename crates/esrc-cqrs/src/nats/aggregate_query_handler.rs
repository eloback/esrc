use std::marker::PhantomData;

use esrc::aggregate::{Aggregate, Root};
use esrc::error;
use esrc::event::replay::ReplayOneExt;
use esrc::nats::NatsStore;
use esrc::version::DeserializeVersion;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::query::QueryHandler;

/// A standard query envelope sent over NATS.
///
/// The query payload wraps only the aggregate ID because the handler already
/// knows which aggregate type and query to execute. If a query requires
/// additional parameters they can be placed alongside `id` in a custom
/// request type by implementing [`QueryHandler`] directly.
#[derive(Debug, Deserialize, Serialize)]
pub struct QueryEnvelope {
    /// The ID of the aggregate instance to query.
    pub id: Uuid,
}

/// A standard reply envelope returned after processing a query.
///
/// On success the inner `data` field contains the serialized response value
/// (a JSON object). On failure `success` is false and `error` is set.
#[derive(Debug, Deserialize, Serialize)]
pub struct QueryReply {
    /// Whether the query succeeded.
    pub success: bool,
    /// The query result serialized as a JSON value, present when `success` is true.
    pub data: Option<serde_json::Value>,
    /// The structured CQRS error, present only when `success` is false.
    pub error: Option<crate::Error>,
}

/// A projection function that maps an aggregate root to the query response type.
///
/// This is a plain function pointer so that `AggregateQueryHandler` remains
/// `Send + Sync` without requiring a boxed closure.
pub type ProjectFn<A, R> = fn(&Root<A>) -> R;

/// A generic [`QueryHandler`] implementation for NATS-backed aggregates.
///
/// This handler:
/// 1. Deserializes the incoming payload as a [`QueryEnvelope`].
/// 2. Loads the aggregate using [`ReplayOneExt::read`].
/// 3. Applies the user-supplied projection function to produce the response.
/// 4. Returns a serialized [`QueryReply`] containing the response as JSON.
///
/// `A` is the aggregate type. `A::Event` must implement `DeserializeVersion`.
/// `R` is the response / read-model type and must implement `Serialize`.
///
/// # Example
///
/// ```rust,ignore
/// let handler = AggregateQueryHandler::<MyAggregate, MyReadModel>::new(
///     "MyAggregate.GetState",
///     |root| MyReadModel::from(Root::state(root)),
/// );
/// ```
///
/// # Extending Queries
///
/// If a query requires parameters beyond the aggregate ID, implement
/// [`crate::query::QueryHandler`] directly and deserialize a custom request
/// envelope inside `handle`. Register the custom handler with
/// [`crate::CqrsRegistry::register_query`] as usual.
pub struct AggregateQueryHandler<A, R>
where
    A: Aggregate,
    R: Serialize,
{
    /// The name used to route queries to this handler.
    handler_name: &'static str,
    /// The projection function applied to the loaded aggregate root.
    project: ProjectFn<A, R>,
    _phantom: PhantomData<(A, R)>,
}

impl<A, R> AggregateQueryHandler<A, R>
where
    A: Aggregate,
    R: Serialize,
{
    /// Create a new handler with the given routing name and projection function.
    pub fn new(handler_name: &'static str, project: ProjectFn<A, R>) -> Self {
        Self {
            handler_name,
            project,
            _phantom: PhantomData,
        }
    }
}

impl<A, R> QueryHandler<NatsStore> for AggregateQueryHandler<A, R>
where
    A: Aggregate + Send + Sync + 'static,
    A::Event: DeserializeVersion + Send,
    R: Serialize + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        self.handler_name
    }

    async fn handle<'a>(
        &'a self,
        store: &'a NatsStore,
        payload: &'a [u8],
    ) -> error::Result<Vec<u8>> {
        let envelope: QueryEnvelope =
            serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;

        let root: Root<A> = store.read(envelope.id).await?;

        let response_value = serde_json::to_value((self.project)(&root))
            .map_err(|e| esrc::error::Error::Format(e.into()))?;

        let reply = QueryReply {
            success: true,
            data: Some(response_value),
            error: None,
        };

        serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
    }
}
