====
>   Info: Duration: 22s | ~$0.0422 | Prompt Tokens: 7,665 (cached: 0) | Completion Tokens: 1,536 
>  Model: gpt-5.4 (gpt-5.4-2026-03-05) | Adapter: OpenAIResp
>  Files: Context Files: 4 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git commit -a -m ". command_service - Update command service documentation"
</suggested_git_command>

<FILE_CHANGES>
<FILE_PATCH file_path="src/event.rs">
@@
-/// Serve aggregates as command-handling service endpoints.
+/// Command handling service traits and utilities.
 pub mod command_service;
</FILE_PATCH>

<FILE_PATCH file_path="src/event/command_service.rs">
@@
-/// Serve an aggregate as a NATS service endpoint, processing commands.
+/// Serve aggregates as command-handling service endpoints.
 ///
-/// Implementations listen for incoming command requests, deserialize the
-/// command, load the aggregate via event replay, process the command,
-/// publish the resulting event, and reply to the caller.
+/// Implementations are responsible for receiving serialized commands,
+/// loading the targeted aggregate from the event store, invoking aggregate
+/// command handling, persisting any produced event, and returning a reply to
+/// the caller that indicates success or failure.
 #[trait_variant::make(Send)]
 pub trait CommandService {
@@
-    /// Block and serve commands for the given aggregate type.
+    /// Start serving commands for the given aggregate type.
     ///
-    /// The service listens on a subject derived from `A::Event::name()` with
-    /// a wildcard for the aggregate UUID (e.g. `<event_name>.*`). On each
-    /// request the aggregate is loaded from sequence 0 via `read`, the
-    /// command is deserialized and processed via `try_write`, and the caller
-    /// receives an empty reply on success or a [`CommandError`] on failure.
+    /// Implementations typically keep running until the underlying transport
+    /// is closed or an unrecoverable error occurs.
     ///
-    /// This method runs until the underlying transport is closed or an
-    /// unrecoverable error occurs.
+    /// The exact transport mapping is implementation specific. For example, a
+    /// backend may derive an endpoint subject from `A::Event::name()`, decode
+    /// incoming command payloads into `A::Command`, reconstruct the aggregate
+    /// through replay, and then call into aggregate command handling before
+    /// replying to the requester.
     fn serve<A>(&self) -> impl Future<Output = error::Result<()>> + Send
     where
         A: crate::aggregate::Aggregate + Send + Sync + 'static,
@@
 #[trait_variant::make(Send)]
 pub trait CommandClient {
-    /// Send a command to the service and await the response.
+    /// Send a command for an aggregate instance and await the response.
     ///
-    /// The command is serialized and sent to the subject derived from
-    /// `A::Event::name()` with the aggregate UUID (e.g. `<event_name>.<id>`).
-    /// The caller awaits a reply, which is empty on success or contains a
-    /// [`CommandError`] on failure.
+    /// Implementations serialize `command`, route it to the service endpoint
+    /// associated with aggregate `A` and aggregate identifier `id`, then wait
+    /// for the service reply.
+    ///
+    /// On success this returns `Ok(())`. Transport failures, serialization
+    /// issues, optimistic concurrency conflicts, and aggregate-defined command
+    /// errors are returned as [`error::Error`].
     async fn send_command<A>(&self, id: uuid::Uuid, command: A::Command) -> error::Result<()>
     where
         A: crate::aggregate::Aggregate + Send + Sync + 'static,
</FILE_PATCH>

<FILE_PATCH file_path="src/nats.rs">
@@
-/// Command service implementation for NatsStore.
+/// NATS-backed command service support for `NatsStore`.
 pub mod command_service;
</FILE_PATCH>

<FILE_PATCH file_path="src/nats/command_service.rs">
@@
 #[derive(Debug, Serialize, Deserialize)]
 pub enum ReplyError<Err> {
     /// An error occurred while sending the reply, e.g. a transport error.
     Internal(String),
+    /// The command could not be persisted because the expected stream state
+    /// no longer matched the actual state.
     /// An error occurred while serializing the error reply.
     Conflict,
+    /// The aggregate rejected the command with a domain-specific error.
     External(Err),
 }
 
 #[derive(Debug, Serialize, Deserialize)]
 pub struct CommandReply<E> {
+    /// `None` indicates success. `Some(...)` contains the failure returned by
+    /// the command service.
     pub error: Option<ReplyError<E>>,
 }
@@
 impl NatsStore {
     /// Spawn `serve` as a background task integrated with graceful shutdown.
     ///
-    /// This wraps [`CommandService::serve`] in a tracked, cancellable task
-    /// using the `GracefulShutdown` / `TaskTracker` already present on
-    /// `NatsStore`. The task will be cancelled when
-    /// [`NatsStore::wait_graceful_shutdown`] is called.
+    /// This wraps [`CommandService::serve`] in a tracked, cancellable task
+    /// using the `GracefulShutdown` / `TaskTracker` already present on
+    /// `NatsStore`. The spawned task registers a shutdown trigger, serves
+    /// commands for aggregate `A`, and exits either when serving fails or
+    /// when [`NatsStore::wait_graceful_shutdown`] requests cancellation.
     pub fn spawn_service<A>(&self)
@@
-    #[instrument(skip_all, level = "debug")]
+    /// Process a single incoming NATS service request as an aggregate command.
+    ///
+    /// The request subject is expected to end with the aggregate UUID. The
+    /// payload must contain a JSON-encoded `A::Command`. The aggregate is
+    /// reconstructed through replay, the command is executed via
+    /// [`crate::event::publish::PublishExt::try_write`], and the result is
+    /// converted into a serializable [`CommandReply`].
+    #[instrument(skip_all, level = "debug")]
     pub async fn handle_request<A>(&self, request: &Request) -> CommandReply<A::Error>
     where
         A: Aggregate + Send + Sync + 'static,
@@
 impl CommandClient for NatsStore {
+    /// Send a command over NATS request/reply to the command service for `A`.
+    ///
+    /// The request subject is formed as `<event_name>.<aggregate_id>`, where
+    /// `<event_name>` is `A::Event::name()`. The command is JSON serialized,
+    /// sent as a request, and the JSON reply is decoded into
+    /// [`CommandReply`].
+    ///
+    /// Reply errors are mapped back into the crate's [`crate::error::Error`]
+    /// variants.
     async fn send_command<A>(&self, id: uuid::Uuid, command: A::Command) -> error::Result<()>
     where
         A: crate::aggregate::Aggregate + Send + Sync + 'static,
</FILE_PATCH>
</FILE_CHANGES>



