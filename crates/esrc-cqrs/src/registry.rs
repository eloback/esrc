use std::sync::Arc;

use tokio::task::JoinSet;
use tracing::instrument;

use esrc::error;

use crate::command::CommandHandler;
use crate::projector::ProjectorHandler;

/// A registry that holds command handlers and event projectors.
///
/// `S` is the event store type shared across all handlers. The store must be
/// `Clone` so that each command handler and projector can hold its own handle.
pub struct CqrsRegistry<S> {
    store: S,
    command_handlers: Vec<Arc<dyn ErasedCommandHandler<S>>>,
    projector_handlers: Vec<Arc<dyn ErasedProjectorHandler<S>>>,
}

impl<S> CqrsRegistry<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Create a new registry backed by the given store.
    pub fn new(store: S) -> Self {
        Self {
            store,
            command_handlers: Vec::new(),
            projector_handlers: Vec::new(),
        }
    }

    /// Register a command handler.
    ///
    /// The handler will be invoked when a command matching its `name()` is
    /// dispatched through the registry's command listener.
    pub fn register_command<H>(mut self, handler: H) -> Self
    where
        H: CommandHandler<S> + 'static,
    {
        self.command_handlers.push(Arc::new(handler));
        self
    }

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

    /// Return a reference to the registered command handlers.
    pub fn command_handlers(&self) -> &[Arc<dyn ErasedCommandHandler<S>>] {
        &self.command_handlers
    }

    /// Return a reference to the registered projector handlers.
    pub fn projector_handlers(&self) -> &[Arc<dyn ErasedProjectorHandler<S>>] {
        &self.projector_handlers
    }

    /// Return a clone of the backing store.
    pub fn store(&self) -> S {
        self.store.clone()
    }

    /// Start all projectors as background tasks and return a [`JoinSet`].
    ///
    /// Each projector is spawned on the Tokio runtime. The caller can await
    /// the `JoinSet` to wait for all projectors to complete (or fail).
    #[instrument(skip_all, level = "debug")]
    pub async fn run_projectors(&self) -> error::Result<JoinSet<error::Result<()>>> {
        let mut set = JoinSet::new();

        for handler in &self.projector_handlers {
            let handler = Arc::clone(handler);
            let store = self.store.clone();
            set.spawn(async move { handler.run_erased(&store).await });
        }

        Ok(set)
    }
}

// -- Object-safe erased traits so we can store heterogeneous handlers --

/// Object-safe wrapper for [`CommandHandler`].
pub trait ErasedCommandHandler<S>: Send + Sync + 'static {
    /// The name of the command this handler processes.
    fn name(&self) -> &'static str;
    /// Handle the raw payload and return a reply.
    fn handle_erased<'a>(
        &'a self,
        store: &'a mut S,
        payload: &'a [u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<Vec<u8>>> + Send + 'a>>;
}

impl<S, H> ErasedCommandHandler<S> for H
where
    H: CommandHandler<S> + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        CommandHandler::name(self)
    }

    fn handle_erased<'a>(
        &'a self,
        store: &'a mut S,
        payload: &'a [u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<Vec<u8>>> + Send + 'a>>
    {
        Box::pin(self.handle(store, payload))
    }
}

/// Object-safe wrapper for [`ProjectorHandler`].
pub trait ErasedProjectorHandler<S>: Send + Sync + 'static {
    /// The durable name of this projector.
    fn name(&self) -> &'static str;
    /// Run the projector against the store.
    fn run_erased<'a>(
        &'a self,
        store: &'a S,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = error::Result<()>> + Send + 'a>>;
}

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
