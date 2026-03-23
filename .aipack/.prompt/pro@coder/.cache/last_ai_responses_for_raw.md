====
>   Info: Duration: 58s | ~$0.1724 | Prompt Tokens: 35,272 (cached: 0, cache_creation: 0) | Completion Tokens: 4,442 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 32 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll execute the next step from the plan: "Add ServiceCommandReply helper and CqrsClient dispatch method".

This step involves:
1. Adding `ServiceCommandReply<R>` struct to `service_command_handler.rs`
2. Adding `dispatch_service_command` method to `CqrsClient`
3. Updating `lib.rs` re-exports

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command/service_command_handler.rs">
@@
+use serde::{de::DeserializeOwned, Deserialize, Serialize};
+
+use crate::command::NatsServiceCommandHandler;
+use crate::registry::ErasedCommandHandler;
+use esrc::error::{self, Error};
+
+/// A convenience reply envelope for service command handlers.
+///
+/// This is an opt-in type; users may return any serializable bytes from their
+/// [`NatsServiceCommandHandler::handle`] implementation. When used, `R` should
+/// match the type that the caller expects to deserialize on the other side.
+#[derive(Debug, Serialize, Deserialize)]
+pub struct ServiceCommandReply<R> {
+    /// Whether the command succeeded.
+    pub success: bool,
+    /// The response data, present when `success` is true.
+    pub data: Option<R>,
+    /// The structured CQRS error, present only when `success` is false.
+    pub error: Option<crate::Error>,
+}
+
+impl ServiceCommandReply<()> {
+    /// Construct a successful reply with no data payload.
+    pub fn ok() -> Self {
+        Self {
+            success: true,
+            data: Some(()),
+            error: None,
+        }
+    }
+}
+
+impl<R> ServiceCommandReply<R> {
+    /// Construct a successful reply carrying the given data.
+    pub fn ok_with(data: R) -> Self {
+        Self {
+            success: true,
+            data: Some(data),
+            error: None,
+        }
+    }
+
+    /// Construct a failure reply carrying the given CQRS error.
+    pub fn err(e: crate::Error) -> Self {
+        Self {
+            success: false,
+            data: None,
+            error: Some(e),
+        }
+    }
+}
+
+/// Adapter that wraps a [`NatsServiceCommandHandler`] into a [`crate::command::CommandHandler`].
+///
+/// All variants of `C` are received under a single NATS endpoint named by
+/// [`NatsServiceCommandHandler::name`]. The raw payload is deserialized as `C`
+/// and forwarded to the inner handler. The reply bytes are returned verbatim.
+pub struct ServiceCommandHandler<H, C> {
+    handler: H,
+    name: &'static str,
+    _marker: std::marker::PhantomData<C>,
+}
+
+impl<H, C> ServiceCommandHandler<H, C>
+where
+    C: Serialize + DeserializeOwned + Send + Sync + 'static,
+{
+    /// Create a new adapter wrapping the given handler.
+    ///
+    /// The endpoint name is taken from [`NatsServiceCommandHandler::name`] so
+    /// there is a single registration call.
+    pub fn new<S>(handler: H) -> Self
+    where
+        H: NatsServiceCommandHandler<S, C>,
+    {
+        let name = handler.name();
+        Self {
+            handler,
+            name,
+            _marker: std::marker::PhantomData,
+        }
+    }
+}
+
+impl<S, H, C> crate::command::CommandHandler<S> for ServiceCommandHandler<H, C>
+where
+    S: Send + Sync + 'static,
+    H: NatsServiceCommandHandler<S, C> + Send + Sync + 'static,
+    C: Serialize + DeserializeOwned + Send + Sync + 'static,
+{
+    fn name(&self) -> &'static str {
+        self.name
+    }
+
+    async fn handle<'a>(
+        &'a self,
+        store: &'a mut S,
+        payload: &'a [u8],
+    ) -> error::Result<Vec<u8>> {
+        let command: C =
+            serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;
+        self.handler.handle(store, command).await
+    }
+}
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command/mod.rs">
@@
-/// Aggregate command handler wiring: maps a typed command to an aggregate and writes events.
-pub mod aggregate_command_handler;
-/// Service command handler adapter: wraps a NatsServiceCommandHandler into CommandHandler.
-pub mod service_command_handler;
-
-pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
-pub use service_command_handler::ServiceCommandHandler;
+/// Aggregate command handler wiring: maps a typed command to an aggregate and writes events.
+pub mod aggregate_command_handler;
+/// Service command handler adapter: wraps a NatsServiceCommandHandler into CommandHandler.
+pub mod service_command_handler;
+
+pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
+pub use service_command_handler::{ServiceCommandHandler, ServiceCommandReply};
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
 pub use command::ServiceCommandHandler;
+pub use command::service_command_handler::ServiceCommandReply;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/lib.rs">
@@
 pub use command::CommandHandler;
 pub use command::NatsServiceCommandHandler;
 pub use error::Error;
 #[cfg(feature = "nats")]
 pub use nats::client::CqrsClient;
 #[cfg(feature = "nats")]
 pub use nats::command::ServiceCommandHandler;
+#[cfg(feature = "nats")]
+pub use nats::command::service_command_handler::ServiceCommandReply;
 pub use projector::ProjectorHandler;
 pub use registry::CqrsRegistry;
 pub use query::QueryHandler;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/client/cqrs_client.rs">
@@
 use async_nats::Client;
 use serde::{Deserialize, Serialize};
 use uuid::Uuid;
 
 use crate::error::{self, Error};
 
 use crate::nats::command::aggregate_command_handler::CommandEnvelope;
 use crate::nats::command_dispatcher::{command_subject, CommandReply};
 use crate::nats::query_dispatcher::{query_subject, QueryEnvelope, QueryReply};
@@
     /// Send a query and deserialize the result directly into `T`.
     ///
     /// This is a convenience wrapper around [`send_query`] for callers that
     /// want a typed result rather than a raw [`QueryReply`]. Returns
     /// `Err(Error::Internal(...))` when `reply.success` is false.
     ///
     /// # Errors
     ///
     /// Returns an error if the NATS request fails, the reply cannot be
     /// deserialized, or the `data` field is absent even though `success` is true.
     ///
     /// [`send_query`]: CqrsClient::send_query
     pub async fn dispatch_query<T>(
         &self,
         service_name: &str,
         handler_name: &str,
         id: Uuid,
     ) -> error::Result<T>
     where
         T: for<'de> Deserialize<'de>,
     {
         let reply = self.send_query(service_name, handler_name, id).await?;
         if !reply.success {
             let err = reply
                 .error
                 .unwrap_or(Error::Internal("query failed but no error present".into()));
             return Err(err);
         }
         let data = reply
             .data
             .ok_or_else(|| Error::Internal("query succeeded but returned no data".into()))?;
         serde_json::from_value::<T>(data).map_err(|e| Error::Format(e.to_string()))
     }
+
+    /// Send a typed service command and deserialize the raw reply as `R`.
+    ///
+    /// The command is serialized and sent to subject `<service_name>.<command_name>`.
+    /// The reply bytes are deserialized directly as `R`, so `R` must match whatever
+    /// the handler returns (e.g., [`crate::ServiceCommandReply<D>`] for a structured
+    /// reply, or any other serializable type).
+    ///
+    /// This method is intentionally low-level: use [`dispatch_service_command`] when
+    /// the handler uses [`crate::ServiceCommandReply`] and you want `Result<D>`.
+    ///
+    /// [`dispatch_service_command`]: CqrsClient::dispatch_service_command
+    pub async fn send_service_command<C, R>(
+        &self,
+        service_name: &str,
+        command_name: &str,
+        command: C,
+    ) -> error::Result<R>
+    where
+        C: Serialize,
+        R: for<'de> Deserialize<'de>,
+    {
+        let payload = serde_json::to_vec(&command).map_err(|e| Error::Format(e.to_string()))?;
+        let subject = command_subject(service_name, command_name);
+
+        let msg = self
+            .inner
+            .request(subject, payload.into())
+            .await
+            .map_err(|e| Error::Internal(e.to_string()))?;
+
+        serde_json::from_slice::<R>(&msg.payload).map_err(|e| Error::Format(e.to_string()))
+    }
+
+    /// Send a typed service command and return the inner data on success.
+    ///
+    /// Sends `command` to subject `<service_name>.<command_name>` and expects the
+    /// handler to reply with a [`crate::ServiceCommandReply<R>`]. On success,
+    /// returns `Ok(reply.data)`. On failure, returns `Err` with the reply error.
+    ///
+    /// # Errors
+    ///
+    /// Returns an error if the NATS request fails, the reply cannot be deserialized,
+    /// or `reply.success` is false.
+    pub async fn dispatch_service_command<C, R>(
+        &self,
+        service_name: &str,
+        command_name: &str,
+        command: C,
+    ) -> error::Result<R>
+    where
+        C: Serialize,
+        R: for<'de> Deserialize<'de>,
+    {
+        use crate::nats::command::service_command_handler::ServiceCommandReply;
+
+        let reply = self
+            .send_service_command::<C, ServiceCommandReply<R>>(
+                service_name,
+                command_name,
+                command,
+            )
+            .await?;
+
+        if reply.success {
+            reply
+                .data
+                .ok_or_else(|| Error::Internal("command succeeded but returned no data".into()))
+        } else {
+            let err = reply
+                .error
+                .unwrap_or(Error::Internal("command failed but no error present".into()));
+            Err(err)
+        }
+    }
 }
 
 impl From<Client> for CqrsClient {
</FILE_PATCH>

<FILE_PATCH file_path=".aipack/.prompt/pro@coder/dev/plan/plan-1-todo-steps.md">
@@
-## Step - Add ServiceCommandReply helper and CqrsClient dispatch method
-      status: not_started
-time-created: 2026-03-23 18:55:03
-
-References: see the definition in plan-1-todo-steps.md,
-step 'Step - Define NatsServiceCommandHandler trait and adapter'.
-
-- In `crates/esrc-cqrs/src/nats/command/service_command_handler.rs`:
-  - Add `ServiceCommandReply<R>` struct (generic over `R: Serialize + DeserializeOwned`) with
-    fields `success: bool`, `data: Option<R>`, `error: Option<crate::Error>`.
-  - Derive `Serialize`, `Deserialize`, `Debug`.
-  - This is an opt-in convenience type; the user may use it or return any other serializable
-    bytes from their `handle` implementation.
-  - Add a `ServiceCommandReply::<()>::ok()` constructor and `ServiceCommandReply::err(e)` constructor.
-
-- In `crates/esrc-cqrs/src/nats/client/cqrs_client.rs`:
-  - Add method `dispatch_service_command<C, R>(&self, service_name, command_name, command: C) -> Result<R>`
-    where `C: Serialize`, `R: DeserializeOwned`.
-    - Sends the serialized command to subject `<service_name>.<command_name>`.
-    - Deserializes the raw reply bytes as `R` and returns it.
-  - This is intentionally low-level: the user chooses `R` to match whatever their handler returns.
-
-- Update `crates/esrc-cqrs/src/lib.rs` re-exports to expose `ServiceCommandReply`.
-
 ## Step - Add cafe example extension and integration tests for ServiceCommandHandler
</FILE_PATCH>

<FILE_NEW file_path=".aipack/.prompt/pro@coder/dev/plan/plan-2-active-step.md">
## Step - Add ServiceCommandReply helper and CqrsClient dispatch method
      status: active
time-created: 2026-03-23 18:55:03
time-current: 2026-03-23 19:08:14

References: see the definition in plan-3-done-steps.md,
step 'Step - Define NatsServiceCommandHandler trait and adapter'.

- In `crates/esrc-cqrs/src/nats/command/service_command_handler.rs`:
  - Add `ServiceCommandReply<R>` struct (generic over `R: Serialize + DeserializeOwned`) with
    fields `success: bool`, `data: Option<R>`, `error: Option<crate::Error>`.
  - Derive `Serialize`, `Deserialize`, `Debug`.
  - This is an opt-in convenience type; the user may use it or return any other serializable
    bytes from their `handle` implementation.
  - Add a `ServiceCommandReply::<()>::ok()` constructor and `ServiceCommandReply::err(e)` constructor.

- In `crates/esrc-cqrs/src/nats/client/cqrs_client.rs`:
  - Add method `dispatch_service_command<C, R>(&self, service_name, command_name, command: C) -> Result<R>`
    where `C: Serialize`, `R: DeserializeOwned`.
    - Sends the serialized command to subject `<service_name>.<command_name>`.
    - Deserializes the raw reply bytes as `R` and returns it.
  - This is intentionally low-level: the user chooses `R` to match whatever their handler returns.

- Update `crates/esrc-cqrs/src/lib.rs` re-exports to expose `ServiceCommandReply`.

### Implementation Considerations

- `ServiceCommandReply<()>` gets a dedicated `ok()` constructor (no data), while the generic
  `ok_with(data: R)` constructor covers cases where data needs to be returned.
- `send_service_command` is the low-level raw variant; `dispatch_service_command` unwraps the
  `ServiceCommandReply` envelope and returns `Result<R>` for ergonomic use.
- The `ServiceCommandHandler` struct was also created in this step since it was absent from the
  prior active step's implementation (the file existed but was empty). It now contains the full
  adapter wiring: deserialization of `C` from raw bytes, delegation to the inner handler.
</FILE_NEW>

<FILE_PATCH file_path=".aipack/.prompt/pro@coder/dev/plan/plan-3-done-steps.md">
@@
 ## Step - Define NatsServiceCommandHandler trait and adapter
       status: done
 time-created: 2026-03-23 18:55:03
    time-done: 2026-03-23 19:01:15
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs - Add ServiceCommandReply helper and CqrsClient dispatch_service_command method"
</suggested_git_command>

