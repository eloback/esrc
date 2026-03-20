====
>   Info: Duration: 31s | ~$0.073 | Prompt Tokens: 13,103 (cached: 0, cache_creation: 0) | Completion Tokens: 2,246 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 8 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

The current design is inconsistent: success replies use `CommandReply` (a structured JSON body) while errors bypass that entirely and use the NATS service-level error mechanism (HTTP-like 500 status with a plain string). This means callers must handle two completely different response shapes.

The best approach is to always return a `CommandReply` JSON body, even on failure, and stop using the NATS service error path for command handler errors. This way:

- Callers always deserialize the same `CommandReply` struct.
- `success: false` signals failure, and `message` carries the error description.
- The NATS service error path is reserved for truly unrecoverable infrastructure problems (e.g., serializing the reply itself fails), not for domain or format errors.

The changes needed are:

1. `command_dispatcher.rs` - respond with a `CommandReply` failure body instead of a NATS service error.
2. `aggregate_command_handler.rs` - on error, return a `CommandReply` with `success: false` rather than propagating `Err`.

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command_dispatcher.rs">
@@
-use esrc::error::{self, Error};
+use esrc::error::{self, Error};
+use super::aggregate_command_handler::CommandReply;
@@
                     let payload = request.message.payload.as_ref();
                     match handler.handle_erased(&mut store, payload).await {
                         Ok(reply) => {
                             let _ = request.respond(Ok(reply.into())).await;
                         },
                         Err(e) => {
-                            // Send an error reply so the caller is not left waiting.
-                            let msg = format!("{e}");
-                            let _ = request
-                                .respond(Err(async_nats::service::error::Error {
-                                    code: 500,
-                                    status: msg,
-                                }))
-                                .await;
+                            // Encode the failure as a CommandReply so callers always
+                            // deserialize the same shape regardless of outcome.
+                            let failure = CommandReply {
+                                id: uuid::Uuid::nil(),
+                                success: false,
+                                message: Some(format!("{e}")),
+                            };
+                            let body = serde_json::to_vec(&failure).unwrap_or_default();
+                            let _ = request.respond(Ok(body.into())).await;
                         },
                     }
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/aggregate_command_handler.rs">
@@
+use uuid::Uuid;
-use uuid::Uuid;
@@
     async fn handle<'a>(
         &'a self,
         store: &'a mut NatsStore,
         payload: &'a [u8],
     ) -> error::Result<Vec<u8>> {
         let envelope: CommandEnvelope<A::Command> =
             serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;
 
         let root: Root<A> = store.read(envelope.id).await?;
-        let root = store.try_write(root, envelope.command, None).await?;
+        let agg_id = envelope.id;
+        let root = store.try_write(root, envelope.command, None).await;
 
-        let reply = CommandReply {
-            id: Root::id(&root),
-            success: true,
-            message: None,
-        };
-        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
+        let reply = match root {
+            Ok(written) => CommandReply {
+                id: Root::id(&written),
+                success: true,
+                message: None,
+            },
+            Err(e) => CommandReply {
+                id: agg_id,
+                success: false,
+                message: Some(format!("{e}")),
+            },
+        };
+        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
     }
</FILE_PATCH>

</FILE_CHANGES>

Wait, I have a duplicate `use uuid::Uuid;` in the patch above. Let me re-examine, the original file already has `use uuid::Uuid;` so I should not add it again, and the dispatcher needs the import. Let me redo cleanly.

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command_dispatcher.rs">
@@
 use esrc::error::{self, Error};
 
 use crate::registry::ErasedCommandHandler;
+use super::aggregate_command_handler::CommandReply;
@@
                     let payload = request.message.payload.as_ref();
                     match handler.handle_erased(&mut store, payload).await {
                         Ok(reply) => {
                             let _ = request.respond(Ok(reply.into())).await;
                         },
                         Err(e) => {
-                            // Send an error reply so the caller is not left waiting.
-                            let msg = format!("{e}");
-                            let _ = request
-                                .respond(Err(async_nats::service::error::Error {
-                                    code: 500,
-                                    status: msg,
-                                }))
-                                .await;
+                            // Encode the failure as a CommandReply so callers always
+                            // deserialize the same shape regardless of outcome.
+                            let failure = CommandReply {
+                                id: uuid::Uuid::nil(),
+                                success: false,
+                                message: Some(format!("{e}")),
+                            };
+                            let body = serde_json::to_vec(&failure).unwrap_or_default();
+                            let _ = request.respond(Ok(body.into())).await;
                         },
                     }
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/aggregate_command_handler.rs">
@@
     async fn handle<'a>(
         &'a self,
         store: &'a mut NatsStore,
         payload: &'a [u8],
     ) -> error::Result<Vec<u8>> {
         let envelope: CommandEnvelope<A::Command> =
             serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;
 
         let root: Root<A> = store.read(envelope.id).await?;
-        let root = store.try_write(root, envelope.command, None).await?;
+        let agg_id = envelope.id;
+        let root = store.try_write(root, envelope.command, None).await;
 
-        let reply = CommandReply {
-            id: Root::id(&root),
-            success: true,
-            message: None,
-        };
-        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
+        let reply = match root {
+            Ok(written) => CommandReply {
+                id: Root::id(&written),
+                success: true,
+                message: None,
+            },
+            Err(e) => CommandReply {
+                id: agg_id,
+                success: false,
+                message: Some(format!("{e}")),
+            },
+        };
+        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
     }
</FILE_PATCH>

</FILE_CHANGES>

Key design decisions made:

- `aggregate_command_handler.rs`: `try_write` errors (domain errors, OCC conflicts, external errors) are now captured and returned as a `CommandReply { success: false, message: Some(...) }` instead of propagating as `Err`. Deserialization and replay errors still propagate as `Err` since they represent infrastructure failures, not domain outcomes.
- `command_dispatcher.rs`: The `Err` arm (now only reachable for infrastructure-level failures like replay or serialization errors) also returns a `CommandReply` body over the success channel, so callers never need to handle the NATS service error path for command results. The `id` is set to `Uuid::nil()` in that case since no aggregate ID is available.
- Callers always get a `CommandReply` JSON body and check `success` to determine outcome, never needing to handle NATS-level error replies differently.

<suggested_git_command>
git commit -a -m "^ esrc-cqrs - Always return CommandReply on error instead of NATS service error"
</suggested_git_command>

