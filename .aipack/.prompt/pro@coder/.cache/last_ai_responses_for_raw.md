====
>   Info: Duration: 1m 5s | ~$0.1016 | Prompt Tokens: 11,487 (cached: 0, cache_creation: 0) | Completion Tokens: 4,479 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 10 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Here is my plan before writing the code:

- Add a `query.rs` module under `crates/esrc-cqrs/src/` defining the `QueryHandler` trait, mirroring `CommandHandler` but typed over a request and a response.
- Add a `nats/aggregate_query_handler.rs` for the NATS-backed generic aggregate query handler (loads a read model / aggregate state and returns it serialized).
- Add a `nats/query_dispatcher.rs` for the NATS service-based query dispatcher (same pattern as command dispatcher).
- Wire everything up in `lib.rs`, `nats/mod.rs`, and `registry.rs`.

The query handler is intentionally kept general: the request type and response type are both user-defined, so it works for read-model projector results, aggregate state queries, or anything else.

<FILE_CHANGES>

<FILE_NEW file_path="crates/esrc-cqrs/src/query.rs">
use std::future::Future;

use esrc::error;

/// A handler for a single query type.
///
/// Implementors receive a raw byte payload (the serialized query request),
/// perform all necessary data retrieval (e.g., loading a read model or
/// replaying aggregate state), and return a serialized response payload.
///
/// The generic parameter `S` is the event store type (e.g., `NatsStore`).
///
/// Queries are read-only by convention: a `QueryHandler` should never write
/// events or mutate aggregate state. The store reference is therefore shared
/// (`&S`) rather than exclusive (`&mut S`).
pub trait QueryHandler<S>: Send + Sync + 'static {
    /// The unique name for this query handler.
    ///
    /// This is used to route incoming query messages to the correct handler.
    /// The convention is `<AggregateName>.<QueryName>` or `<ReadModel>.<QueryName>`.
    fn name(&self) -> &'static str;

    /// Handle a raw incoming query payload, returning a reply payload.
    ///
    /// The handler is responsible for deserializing the query, loading the
    /// required data, and serializing the response. Returning an `Err` will
    /// cause an error reply to be sent to the caller.
    fn handle<'a>(
        &'a self,
        store: &'a S,
        payload: &'a [u8],
    ) -> impl Future<Output = error::Result<Vec<u8>>> + Send + 'a;
}
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/aggregate_query_handler.rs">
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
</FILE_NEW>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/query_dispatcher.rs">
use std::sync::Arc;

use async_nats::service::ServiceExt;
use futures::StreamExt;
use tracing::instrument;

use esrc::error::{self, Error};

use crate::registry::ErasedQueryHandler;

/// Version string for the NATS query service group.
pub const QUERY_SERVICE_VERSION: &str = "0.1.0";

/// NATS query dispatcher.
///
/// Registers all query handlers as endpoints on a single NATS service,
/// using core NATS request/reply. Each handler name becomes one endpoint
/// within the service group named `<service_name>`.
///
/// The store reference is shared (`&S`) across all query handlers because
/// queries are read-only by convention.
pub struct NatsQueryDispatcher {
    /// The NATS client used to create the service.
    client: async_nats::Client,
    /// The service group name (e.g. `"myapp-query"`).
    service_name: String,
}

impl NatsQueryDispatcher {
    /// Create a new dispatcher using the given NATS client and service name.
    pub fn new(client: async_nats::Client, service_name: impl Into<String>) -> Self {
        Self {
            client,
            service_name: service_name.into(),
        }
    }

    /// Start the query dispatcher and listen for incoming queries.
    ///
    /// This method creates one NATS service endpoint per registered query
    /// handler. Each endpoint is named after the handler's `name()`. The
    /// dispatcher runs until an error occurs or the NATS connection is closed.
    ///
    /// The store is shared across all endpoint tasks via [`Arc`] because
    /// query handlers only require a shared reference.
    #[instrument(skip_all, level = "debug")]
    pub async fn run<S>(
        &self,
        store: S,
        handlers: &[Arc<dyn ErasedQueryHandler<S>>],
    ) -> error::Result<()>
    where
        S: Clone + Send + Sync + 'static,
    {
        let store = Arc::new(store);

        let service = self
            .client
            .service_builder()
            .description("esrc-cqrs query dispatcher")
            .start(&self.service_name, QUERY_SERVICE_VERSION)
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        let group = service.group(&self.service_name);

        // Build one endpoint per handler and spawn a task for each.
        let mut tasks = tokio::task::JoinSet::new();

        for handler in handlers {
            let handler = Arc::clone(handler);
            let store = Arc::clone(&store);

            let mut endpoint = group
                .endpoint(handler.name())
                .await
                .map_err(|e| Error::Internal(e.into()))?;

            tasks.spawn(async move {
                while let Some(request) = endpoint.next().await {
                    let payload = request.message.payload.as_ref();
                    match handler.handle_erased(&*store, payload).await {
                        Ok(reply) => {
                            let _ = request.respond(Ok(reply.into())).await;
                        },
                        Err(e) => {
                            use crate::nats::aggregate_query_handler::QueryReply;
                            let failure = QueryReply {
                                success: false,
                                data: None,
                                error: Some(crate::error::Error::Internal(format!("{e}"))),
                            };
                            let body = serde_json::to_vec(&failure).unwrap_or_default();
                            let _ = request.respond(Ok(body.into())).await;
                        },
                    }
                }
                error::Result::Ok(())
            });
        }

        // Wait for all endpoint tasks; return the first error encountered.
        while let Some(result) = tasks.join_next().await {
            result.map_err(|e| Error::Internal(e.into()))??;
        }

        Ok(())
    }
}

/// Build the full NATS subject for a query endpoint.
///
/// Pattern: `<service_name>.<handler_name>`
pub fn query_subject(service_name: &str, handler_name: &str) -> String {
    format!("{service_name}.{handler_name}")
}
</FILE_NEW>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
 pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
-pub use aggregate_projector_handler::DurableProjectorHandler;
+pub use aggregate_projector_handler::DurableProjectorHandler;
+pub use aggregate_query_handler::{AggregateQueryHandler, QueryEnvelope, QueryReply};
 pub use command_dispatcher::NatsCommandDispatcher;
+pub use query_dispatcher::NatsQueryDispatcher;
 pub use projector_runner::NatsProjectorRunner;
 
 mod aggregate_command_handler;
 mod aggregate_projector_handler;
+mod aggregate_query_handler;
+mod query_dispatcher;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/registry.rs">
@@
 use crate::command::CommandHandler;
 use crate::projector::ProjectorHandler;
+use crate::query::QueryHandler;
@@
 pub struct CqrsRegistry<S> {
     store: S,
     command_handlers: Vec<Arc<dyn ErasedCommandHandler<S>>>,
     projector_handlers: Vec<Arc<dyn ErasedProjectorHandler<S>>>,
+    query_handlers: Vec<Arc<dyn ErasedQueryHandler<S>>>,
 }
@@
     pub fn new(store: S) -> Self {
         Self {
             store,
             command_handlers: Vec::new(),
             projector_handlers: Vec::new(),
+            query_handlers: Vec::new(),
         }
     }
@@
     /// Register an event projector handler.
     ///
     /// The projector will be started as a background task when `run` is called.
     pub fn register_projector<H>(mut self, handler: H) -> Self
     where
         H: ProjectorHandler<S> + Sync + 'static,
     {
         self.projector_handlers.push(Arc::new(handler));
         self
     }
+
+    /// Register a query handler.
+    ///
+    /// The handler will be invoked when a query matching its `name()` is
+    /// dispatched through the registry's query listener.
+    pub fn register_query<H>(mut self, handler: H) -> Self
+    where
+        H: QueryHandler<S> + 'static,
+    {
+        self.query_handlers.push(Arc::new(handler));
+        self
+    }
@@
     /// Return a reference to the registered projector handlers.
     pub fn projector_handlers(&self) -> &[Arc<dyn ErasedProjectorHandler<S>>] {
         &self.projector_handlers
     }
+
+    /// Return a reference to the registered query handlers.
+    pub fn query_handlers(&self) -> &[Arc<dyn ErasedQueryHandler<S>>] {
+        &self.query_handlers
+    }
@@
 // -- Object-safe erased traits so we can store heterogeneous handlers --
@@
 impl<S, H> ErasedProjectorHandler<S> for H
 where
     H: ProjectorHandler<S> + Send + Sync + 'static,
 {
     fn name(&self) -> &'static str {
         ProjectorHandler::name(self)
     }
 
     fn run_erased<'a>(
         &'a self,
         store: &'a S,
     ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<()>> + Send + 'a>> {
         Box::pin(self.run(store))
     }
 }
+
+/// Object-safe wrapper for [`QueryHandler`].
+pub trait ErasedQueryHandler<S>: Send + Sync + 'static {
+    /// The name of the query this handler processes.
+    fn name(&self) -> &'static str;
+    /// Handle the raw payload and return a reply.
+    fn handle_erased<'a>(
+        &'a self,
+        store: &'a S,
+        payload: &'a [u8],
+    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<Vec<u8>>> + Send + 'a>>;
+}
+
+impl<S, H> ErasedQueryHandler<S> for H
+where
+    H: QueryHandler<S> + Send + Sync + 'static,
+{
+    fn name(&self) -> &'static str {
+        QueryHandler::name(self)
+    }
+
+    fn handle_erased<'a>(
+        &'a self,
+        store: &'a S,
+        payload: &'a [u8],
+    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<Vec<u8>>> + Send + 'a>>
+    {
+        Box::pin(self.handle(store, payload))
+    }
+}
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/lib.rs">
@@
 /// Event projector handler registry entry.
 pub mod projector;
 /// The top-level CQRS registry that holds and drives all handlers.
 pub mod registry;
+/// Query handler trait and registry entry.
+pub mod query;
@@
 pub use command::CommandHandler;
 pub use error::Error;
 pub use projector::ProjectorHandler;
 pub use registry::CqrsRegistry;
+pub use query::QueryHandler;
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs - Add QueryHandler trait, AggregateQueryHandler, NatsQueryDispatcher, and registry integration"
</suggested_git_command>

