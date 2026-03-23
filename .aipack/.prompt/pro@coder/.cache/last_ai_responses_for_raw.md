====
>   Info: Duration: 48s | ~$0.1996 | Prompt Tokens: 48,132 (cached: 0, cache_creation: 0) | Completion Tokens: 3,678 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 46 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll implement the first step: **Define NatsServiceCommandHandler trait and adapter**.

Let me analyze what needs to be done:

1. Add `NatsServiceCommandHandler<S, C>` trait to `crates/esrc-cqrs/src/command.rs`
2. Create `crates/esrc-cqrs/src/nats/command/service_command_handler.rs`
3. Update `crates/esrc-cqrs/src/nats/command/mod.rs`
4. Update `crates/esrc-cqrs/src/nats/mod.rs`
5. Update `crates/esrc-cqrs/src/lib.rs`
6. Update plan files

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/src/command.rs">
@@
 use std::future::Future;
 
 use esrc::error;
+use serde::{de::DeserializeOwned, Serialize};
 
 /// A handler for a single command type.
@@
 pub trait CommandHandler<S>: Send + Sync + 'static {
@@
     fn handle<'a>(
         &'a self,
         store: &'a mut S,
         payload: &'a [u8],
     ) -> impl Future<Output = error::Result<Vec<u8>>> + Send + 'a;
 }
+
+/// A handler for a typed command dispatched under a single NATS service name.
+///
+/// Unlike [`CommandHandler`], which works on raw byte payloads, this trait
+/// receives an already-deserialized command value `C`. The user controls the
+/// reply shape entirely; the returned `Vec<u8>` is sent back verbatim.
+///
+/// The generic parameter `S` is the event store type (e.g., `NatsStore`).
+/// The generic parameter `C` is the command enum type; it must implement
+/// `Serialize` and `DeserializeOwned` so the dispatcher can decode incoming
+/// NATS payloads before forwarding them here.
+///
+/// # Usage
+///
+/// For registering a single handler that covers an entire API group or vertical
+/// slice, implement this trait and wrap the value with [`crate::nats::command::ServiceCommandHandler`].
+/// All commands are received under the same NATS endpoint named by `name()`.
+pub trait NatsServiceCommandHandler<S, C>: Send + Sync + 'static
+where
+    C: Serialize + DeserializeOwned + Send + Sync,
+{
+    /// The unique NATS service/endpoint name for this handler.
+    ///
+    /// All commands of type `C` are routed to the NATS endpoint with this name.
+    fn name(&self) -> &'static str;
+
+    /// Handle a deserialized command, returning a serialized reply payload.
+    ///
+    /// The reply bytes are sent back to the caller verbatim. Use
+    /// [`crate::nats::command::ServiceCommandReply`] as a convenience reply
+    /// envelope, or return any other serializable structure.
+    fn handle<'a>(
+        &'a self,
+        store: &'a mut S,
+        command: C,
+    ) -> impl Future<Output = error::Result<Vec<u8>>> + Send + 'a;
+}
</FILE_PATCH>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/command/service_command_handler.rs">
use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};

use esrc::error::{self, Error};

use crate::command::{CommandHandler, NatsServiceCommandHandler};

/// An adapter that bridges a [`NatsServiceCommandHandler`] into the generic
/// [`CommandHandler`] interface expected by the registry and dispatcher.
///
/// `ServiceCommandHandler<H, C>` holds a user-defined handler `H` that
/// implements `NatsServiceCommandHandler<S, C>` and exposes it as a single
/// NATS endpoint. All variants of `C` are received under that one endpoint;
/// the adapter deserializes the payload and delegates to the user handler.
///
/// `H` is the user handler type.
/// `C` is the typed command enum or struct; it must be JSON-serializable.
pub struct ServiceCommandHandler<H, C> {
    handler: H,
    _phantom: PhantomData<C>,
}

impl<H, C> ServiceCommandHandler<H, C> {
    /// Create a new adapter wrapping the given handler.
    ///
    /// The endpoint name is taken from `handler.name()` so there is a single
    /// registration call and the handler controls its own routing key.
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            _phantom: PhantomData,
        }
    }
}

impl<S, H, C> CommandHandler<S> for ServiceCommandHandler<H, C>
where
    S: Send + Sync + 'static,
    H: NatsServiceCommandHandler<S, C> + Send + Sync + 'static,
    C: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        self.handler.name()
    }

    async fn handle<'a>(
        &'a self,
        store: &'a mut S,
        payload: &'a [u8],
    ) -> error::Result<Vec<u8>> {
        let command: C =
            serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;
        self.handler.handle(store, command).await
    }
}
</FILE_NEW>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command/mod.rs">
@@
 /// Aggregate command handler wiring: maps a typed command to an aggregate and writes events.
 pub mod aggregate_command_handler;
+/// Service command handler adapter: wraps a NatsServiceCommandHandler into CommandHandler.
+pub mod service_command_handler;
 
-pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
+pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
+pub use service_command_handler::ServiceCommandHandler;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
 pub use durable_projector_handler::DurableProjectorHandler;
+
+pub use command::ServiceCommandHandler;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/lib.rs">
@@
 pub use command::CommandHandler;
+pub use command::NatsServiceCommandHandler;
 pub use error::Error;
 #[cfg(feature = "nats")]
 pub use nats::client::CqrsClient;
+#[cfg(feature = "nats")]
+pub use nats::command::ServiceCommandHandler;
 pub use projector::ProjectorHandler;
 pub use registry::CqrsRegistry;
 pub use query::QueryHandler;
</FILE_PATCH>

<FILE_PATCH file_path=".aipack/.prompt/pro@coder/dev/plan/plan-1-todo-steps.md">
@@
-## Step - Define NatsServiceCommandHandler trait and adapter
-      status: not_started
-time-created: 2026-03-23 18:55:03
-
-Add a `NatsServiceCommandHandler<C>` trait and a `ServiceCommandHandler<H, C>` adapter
-struct to `esrc-cqrs` that allow a user to implement a single struct covering a whole
-vertical slice or API group, dispatching a typed enum of commands under one NATS service
-name, with one endpoint per enum variant (derived from the variant name).
-
-- In `crates/esrc-cqrs/src/command.rs`:
-  - Add trait `NatsServiceCommandHandler<S, C>` where `C: Serialize + DeserializeOwned + Send + Sync`:
-    - Required method `fn name(&self) -> &'static str` returning the NATS service/group name.
-    - Required async method `async fn handle(&self, store: &mut S, command: C) -> error::Result<Vec<u8>>`.
-    - The trait is `Send + Sync + 'static` and generic over the store `S` like `CommandHandler<S>`.
-
-- In `crates/esrc-cqrs/src/nats/command/service_command_handler.rs` (new file):
-  - Define `ServiceCommandHandler<H, C>` adapter struct that:
-    - Holds the user handler `H` (which implements `NatsServiceCommandHandler<S, C>`).
-    - Holds the service name (`&'static str`) as the single NATS endpoint name, so all
-      variants of `C` are received under the same subject.
-    - Implements `CommandHandler<S>` by:
-      1. Deserializing the raw payload as `C` via `serde_json`.
-      2. Delegating to `H::handle(store, command)`.
-      3. Returning the reply bytes verbatim (the user controls the reply shape entirely).
-  - Provide `ServiceCommandHandler::new(handler: H) -> Self` constructor, using
-    `handler.name()` as the endpoint name so there is a single registration call.
-
-- In `crates/esrc-cqrs/src/nats/command/mod.rs`:
-  - Add `pub mod service_command_handler;` and re-export `ServiceCommandHandler`.
-
-- In `crates/esrc-cqrs/src/nats/mod.rs`:
-  - Re-export `ServiceCommandHandler` from `nats::command`.
-
-- In `crates/esrc-cqrs/src/lib.rs`:
-  - Re-export `ServiceCommandHandler` at the crate root.
-
-Intended usage pattern (the registration side):
-```rs
-#[derive(Debug, Serialize, Deserialize, Clone)]
-pub enum Commands {
-    RegistrarEscrituracao { id: Uuid, name: String, date: NaiveDate },
-}
-
-pub struct MyServiceHandler { external_api: ExternalApi }
-
-impl NatsServiceCommandHandler<NatsStore, Commands> for MyServiceHandler {
-    fn name(&self) -> &'static str { "my_api_service" }
-    async fn handle(&self, store: &mut NatsStore, command: Commands) -> error::Result<Vec<u8>> { ... }
-}
-
-registry.register_command(ServiceCommandHandler::new(MyServiceHandler { ... }));
-```
-
 ## Step - Add ServiceCommandReply helper and CqrsClient dispatch method
</FILE_PATCH>

<FILE_NEW file_path=".aipack/.prompt/pro@coder/dev/plan/plan-2-active-step.md">
## Step - Define NatsServiceCommandHandler trait and adapter
      status: active
time-created: 2026-03-23 18:55:03
time-current: 2026-03-23 19:01:15

Add a `NatsServiceCommandHandler<C>` trait and a `ServiceCommandHandler<H, C>` adapter
struct to `esrc-cqrs` that allow a user to implement a single struct covering a whole
vertical slice or API group, dispatching a typed enum of commands under one NATS service
name, with one endpoint per enum variant (derived from the variant name).

- In `crates/esrc-cqrs/src/command.rs`:
  - Add trait `NatsServiceCommandHandler<S, C>` where `C: Serialize + DeserializeOwned + Send + Sync`:
    - Required method `fn name(&self) -> &'static str` returning the NATS service/group name.
    - Required async method `async fn handle(&self, store: &mut S, command: C) -> error::Result<Vec<u8>>`.
    - The trait is `Send + Sync + 'static` and generic over the store `S` like `CommandHandler<S>`.

- In `crates/esrc-cqrs/src/nats/command/service_command_handler.rs` (new file):
  - Define `ServiceCommandHandler<H, C>` adapter struct that:
    - Holds the user handler `H` (which implements `NatsServiceCommandHandler<S, C>`).
    - Holds the service name (`&'static str`) as the single NATS endpoint name, so all
      variants of `C` are received under the same subject.
    - Implements `CommandHandler<S>` by:
      1. Deserializing the raw payload as `C` via `serde_json`.
      2. Delegating to `H::handle(store, command)`.
      3. Returning the reply bytes verbatim (the user controls the reply shape entirely).
  - Provide `ServiceCommandHandler::new(handler: H) -> Self` constructor, using
    `handler.name()` as the endpoint name so there is a single registration call.

- In `crates/esrc-cqrs/src/nats/command/mod.rs`:
  - Add `pub mod service_command_handler;` and re-export `ServiceCommandHandler`.

- In `crates/esrc-cqrs/src/nats/mod.rs`:
  - Re-export `ServiceCommandHandler` from `nats::command`.

- In `crates/esrc-cqrs/src/lib.rs`:
  - Re-export `ServiceCommandHandler` at the crate root.

Intended usage pattern (the registration side):
```rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Commands {
    RegistrarEscrituracao { id: Uuid, name: String, date: NaiveDate },
}

pub struct MyServiceHandler { external_api: ExternalApi }

impl NatsServiceCommandHandler<NatsStore, Commands> for MyServiceHandler {
    fn name(&self) -> &'static str { "my_api_service" }
    async fn handle(&self, store: &mut NatsStore, command: Commands) -> error::Result<Vec<u8>> { ... }
}

registry.register_command(ServiceCommandHandler::new(MyServiceHandler { ... }));
```
</FILE_NEW>

</FILE_CHANGES>

<suggested_git_command>
git add -A .
git commit -m "+ esrc-cqrs - Add NatsServiceCommandHandler trait and ServiceCommandHandler adapter"
</suggested_git_command>

