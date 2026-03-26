use async_nats::service::ServiceExt;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use stream_cancel::Tripwire;
use tracing::instrument;

use crate::error::{self, Error};
use crate::event_modeling::ComponentName;
use crate::query::{Query, QueryClient, QueryHandler, QueryService, QuerySpec};

/// The kind of query request sent over NATS request-reply.
#[derive(Debug, Serialize, Deserialize)]
pub enum QueryRequest<Q, Id> {
    /// Fetch a single read model instance by its identifier.
    GetById(Id),
    /// Execute a custom query.
    Query(Q),
}

/// Serializable error payload returned by the NATS query service.
#[derive(Debug, Serialize, Deserialize)]
pub enum QueryReplyError {
    /// An internal or transport error occurred.
    Internal(String),
}

/// Serializable reply payload for a `get_by_id` request.
#[derive(Debug, Serialize, Deserialize)]
pub struct GetByIdReply<RM> {
    /// The read model instance, or `None` if not found.
    pub result: Option<RM>,
    /// `None` indicates success. `Some(...)` contains the failure.
    pub error: Option<QueryReplyError>,
}

/// Serializable reply payload for a custom query request.
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryReply<R> {
    /// The query response, present when the request succeeded.
    pub result: Option<R>,
    /// `None` indicates success. `Some(...)` contains the failure.
    pub error: Option<QueryReplyError>,
}

impl QueryService for super::NatsStore {
    #[instrument(skip_all, level = "debug")]
    async fn serve<H>(&self, spec: &QuerySpec<H>) -> error::Result<()>
    where
        H: QueryHandler + Send + Sync + 'static,
        H::Query: DeserializeOwned + Sync,
        H::Id: DeserializeOwned,
        <H::Query as Query>::ReadModel: Serialize + Sync,
        <H::Query as Query>::Response: Serialize + Sync,
    {
        let subject = spec.name().query_subject();
        let service_name = spec.name().durable_name();

        let service = self
            .client()
            .service_builder()
            .description(format!("Query service for {service_name}"))
            .start(&service_name, "0.0.1")
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        let group = service.group(&subject);

        let mut endpoint = group
            .endpoint_builder()
            .name("query")
            .add("*")
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        while let Some(request) = endpoint.next().await {
            let payload: QueryRequest<H::Query, H::Id> =
                match serde_json::from_slice(&request.message.payload) {
                    Ok(req) => req,
                    Err(e) => {
                        let reply = ErrorOnly {
                            error: Some(QueryReplyError::Internal(format!(
                                "failed to deserialize query request: {e}"
                            ))),
                        };
                        respond_json(&request, &reply).await;
                        continue;
                    },
                };

            match payload {
                QueryRequest::GetById(id) => {
                    let reply = match spec.handler().get_by_id(id).await {
                        Ok(result) => GetByIdReply {
                            result,
                            error: None,
                        },
                        Err(e) => GetByIdReply {
                            result: None,
                            error: Some(QueryReplyError::Internal(format!("{e}"))),
                        },
                    };
                    respond_json(&request, &reply).await;
                },
                QueryRequest::Query(query) => {
                    let reply = match spec.handler().handle(query).await {
                        Ok(result) => QueryReply {
                            result: Some(result),
                            error: None,
                        },
                        Err(e) => QueryReply {
                            result: None,
                            error: Some(QueryReplyError::Internal(format!("{e}"))),
                        },
                    };
                    respond_json(&request, &reply).await;
                },
            }
        }

        Ok(())
    }
}

impl QueryClient for super::NatsStore {
    async fn get_by_id<Q, Id>(
        &self,
        name: &ComponentName,
        id: Id,
    ) -> error::Result<Option<Q::ReadModel>>
    where
        Q: Query,
        Q::ReadModel: DeserializeOwned,
        Id: Serialize + Send,
    {
        let subject = format!("{}.query", name.query_subject());
        let request: QueryRequest<(), Id> = QueryRequest::GetById(id);
        let payload = serde_json::to_vec(&request).map_err(|e| {
            Error::Internal(format!("failed to serialize get_by_id request: {e}").into())
        })?;

        let message = self
            .client()
            .request(subject, payload.into())
            .await
            .map_err(|e| {
                Error::Internal(format!("failed to send get_by_id request: {e}").into())
            })?;

        let reply: GetByIdReply<Q::ReadModel> =
            serde_json::from_slice(&message.payload).map_err(|e| {
                Error::Internal(format!("failed to deserialize get_by_id reply: {e}").into())
            })?;

        match reply.error {
            None => Ok(reply.result),
            Some(QueryReplyError::Internal(e)) => Err(Error::Internal(e.into())),
        }
    }

    async fn query<Q>(&self, name: &ComponentName, query: Q) -> error::Result<Q::Response>
    where
        Q: Query + Serialize,
        Q::Response: DeserializeOwned,
    {
        let subject = format!("{}.query", name.query_subject());
        let request: QueryRequest<Q, ()> = QueryRequest::Query(query);
        let payload = serde_json::to_vec(&request).map_err(|e| {
            Error::Internal(format!("failed to serialize query request: {e}").into())
        })?;

        let message = self
            .client()
            .request(subject, payload.into())
            .await
            .map_err(|e| Error::Internal(format!("failed to send query request: {e}").into()))?;

        let reply: QueryReply<Q::Response> =
            serde_json::from_slice(&message.payload).map_err(|e| {
                Error::Internal(format!("failed to deserialize query reply: {e}").into())
            })?;

        match reply.error {
            Some(QueryReplyError::Internal(e)) => Err(Error::Internal(e.into())),
            None => reply.result.ok_or_else(|| {
                Error::Internal("query reply contained no result and no error".into())
            }),
        }
    }
}

/// Helper to serialize and send a JSON reply, logging errors if the response fails.
async fn respond_json<T: Serialize>(request: &async_nats::service::Request, reply: &T) {
    let bytes = match serde_json::to_vec(reply) {
        Ok(b) => bytes::Bytes::from(b),
        Err(e) => {
            tracing::error!("failed to serialize query reply: {e}");
            let fallback = ErrorOnly {
                error: Some(QueryReplyError::Internal(format!(
                    "failed to serialize query reply: {e}"
                ))),
            };
            match serde_json::to_vec(&fallback) {
                Ok(b) => bytes::Bytes::from(b),
                Err(e2) => {
                    tracing::error!("failed to serialize fallback error reply: {e2}");
                    return;
                },
            }
        },
    };

    if let Err(e) = request.respond(Ok(bytes)).await {
        tracing::error!("failed to send query reply: {e}");
    }
}

/// A minimal reply type used when only an error needs to be sent back.
#[derive(Debug, Serialize, Deserialize)]
struct ErrorOnly {
    error: Option<QueryReplyError>,
}

impl super::NatsStore {
    /// Spawn `serve` for a query specification as a background task with graceful shutdown.
    ///
    /// This wraps [`QueryService::serve`] in a tracked, cancellable task
    /// using the `GracefulShutdown` / `TaskTracker` already present on
    /// `NatsStore`. The spawned task registers a shutdown trigger, serves
    /// queries for the given specification, and exits either when serving
    /// fails or when [`NatsStore::wait_graceful_shutdown`] requests
    /// cancellation.
    pub fn spawn_query_service<H>(&self, spec: QuerySpec<H>)
    where
        H: QueryHandler + 'static,
        H::Query: DeserializeOwned + Sync,
        H::Id: DeserializeOwned,
        <H::Query as Query>::ReadModel: Serialize + Sync,
        <H::Query as Query>::Response: Serialize + Sync,
    {
        let store = self.clone();
        let service_name = spec.name().durable_name();
        let (trigger, tripwire) = Tripwire::new();

        let exit_tx = self.graceful_shutdown.exit_tx.clone();

        self.graceful_shutdown.task_tracker.spawn(async move {
            // Register the trigger so it is cancelled during graceful shutdown.
            if exit_tx.send(trigger).await.is_err() {
                tracing::warn!("failed to register shutdown trigger for query service");
                return;
            }

            tokio::select! {
                result = store.serve(&spec) => {
                    if let Err(e) = result {
                        tracing::error!(service = %service_name, err=?e, "query service stopped");
                    }
                }
                _ = tripwire => {
                    tracing::info!(service = %service_name, "shutting down query service");
                }
            }
        });
    }
}
