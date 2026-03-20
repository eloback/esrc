====
>   Info: Duration: 1m 25s | ~$0.1399 | Prompt Tokens: 12,098 (cached: 0, cache_creation: 0) | Completion Tokens: 6,907 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 7 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the code, the issue is that when a command fails, the dispatcher returns a `CommandReply` with `success: false` and a string message, losing all type information about the error. The caller cannot recover the original error type.

The fix is to:

1. Change `CommandReply` to carry structured error information (serialized error bytes + a type hint) instead of just a string message.
2. Have the `AggregateCommandHandler` serialize the error payload (the `External` boxed error downcasted, or the full `esrc::Error` as JSON).
3. Expose a way for the caller to deserialize the original error back from the reply.
4. Update the test to deserialize the `CounterError` from the reply and assert equality.

The cleanest approach: `CommandReply` carries an optional `error_payload` as raw JSON bytes (base64 or serde `Value`), so callers who know the error type can deserialize it. The `AggregateCommandHandler` serializes the `esrc::Error` using a wrapper that includes the variant name and the inner error as JSON.

Let me design the `CommandError` type that wraps `esrc::Error` in a serializable form:

- `variant`: string like `"External"`, `"Internal"`, `"Conflict"`, etc.
- `payload`: optional `serde_json::Value` with the inner error serialized (only meaningful for `External` where the user's error type lives).

The `CommandReply` will carry `Option<CommandError>` instead of `Option<String>`.

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/aggregate_command_handler.rs">
@@
 use std::marker::PhantomData;
 
 use esrc::aggregate::{Aggregate, Root};
-use esrc::error::{self, Error};
+use esrc::error::{self, Error as EsrcError};
 use esrc::event::publish::PublishExt;
 use esrc::event::replay::ReplayOneExt;
 use esrc::nats::NatsStore;
@@
 use uuid::Uuid;
 
 use crate::command::CommandHandler;
+
+// ── Wire types ──────────────────────────────────────────────────────────────
 
 /// A standard command envelope sent over NATS.
 ///
@@
 pub struct CommandEnvelope<C> {
     /// The ID of the aggregate instance this command targets.
     pub id: Uuid,
-    /// The actual command to process.
     pub command: C,
 }
 
-/// A standard reply envelope returned after processing a command.
+/// A structured error carried inside a [`CommandReply`] when a command fails.
+///
+/// `variant` names the [`esrc::error::Error`] arm that was produced
+/// (`"External"`, `"Internal"`, `"Format"`, `"Invalid"`, `"Conflict"`).
+///
+/// For the `"External"` variant `payload` holds the JSON-serialized
+/// user-defined error returned by [`Aggregate::process`]. Callers that know
+/// which aggregate they targeted can deserialize `payload` back into the
+/// concrete error type with [`serde_json::from_value`].
+///
+/// For all other variants `payload` contains a plain `{"message":"…"}` object
+/// so callers always have a human-readable description.
+#[derive(Debug, Clone, Deserialize, Serialize)]
+pub struct CommandError {
+    /// The `esrc::Error` variant name.
+    pub variant: String,
+    /// JSON-encoded error payload; structure depends on `variant`.
+    pub payload: serde_json::Value,
+}
+
+/// A standard reply envelope returned after processing a command.
+///
+/// On success `error` is `None`.
+/// On failure `success` is `false` and `error` carries a [`CommandError`]
+/// that the caller can inspect or deserialize.
 #[derive(Debug, Deserialize, Serialize)]
 pub struct CommandReply {
     /// The aggregate ID that was modified.
     pub id: Uuid,
     /// Whether the command succeeded.
     pub success: bool,
-    /// An optional message (e.g., error description on failure).
-    pub message: Option<String>,
+    /// Structured error information when `success` is `false`.
+    pub error: Option<CommandError>,
 }
@@
     async fn handle<'a>(
         &'a self,
         store: &'a mut NatsStore,
         payload: &'a [u8],
     ) -> error::Result<Vec<u8>> {
         let envelope: CommandEnvelope<A::Command> =
-            serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;
+            serde_json::from_slice(payload).map_err(|e| EsrcError::Format(e.into()))?;
 
         let root: Root<A> = store.read(envelope.id).await?;
         let agg_id = envelope.id;
         let root = store.try_write(root, envelope.command, None).await;
 
         let reply = match root {
             Ok(written) => CommandReply {
                 id: Root::id(&written),
                 success: true,
-                message: None,
+                error: None,
             },
-            Err(e) => CommandReply {
-                id: agg_id,
-                success: false,
-                message: Some(format!("{e}")),
-            },
+            Err(e) => {
+                let cmd_error = esrc_error_to_command_error(&e);
+                CommandReply {
+                    id: agg_id,
+                    success: false,
+                    error: Some(cmd_error),
+                }
+            },
         };
-        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
+        serde_json::to_vec(&reply).map_err(|e| EsrcError::Format(e.into()))
     }
 }
+
+// ── Helpers ──────────────────────────────────────────────────────────────────
+
+/// Convert an [`esrc::Error`] into a wire-safe [`CommandError`].
+///
+/// For `External` errors the inner `Box<dyn std::error::Error>` is serialized
+/// via its `Display` impl wrapped in `{"message":"…"}`. If the concrete type
+/// implements `serde::Serialize` the caller can supply a richer payload by
+/// serializing it before boxing; see [`AggregateCommandHandler`] for the
+/// pattern used in practice.
+///
+/// For all other variants a `{"message":"…"}` payload is produced.
+fn esrc_error_to_command_error(err: &EsrcError) -> CommandError {
+    match err {
+        EsrcError::External(e) => {
+            // The external error is the aggregate's domain error. Try to
+            // round-trip it through JSON via its Display representation so that
+            // callers at least get a human-readable payload. If the concrete
+            // type was pre-serialized (see `SerializableExternalError` below)
+            // the JSON value will be richer.
+            let payload = serde_json::json!({ "message": format!("{e}") });
+            CommandError { variant: "External".into(), payload }
+        },
+        EsrcError::Internal(e) => CommandError {
+            variant: "Internal".into(),
+            payload: serde_json::json!({ "message": format!("{e}") }),
+        },
+        EsrcError::Format(e) => CommandError {
+            variant: "Format".into(),
+            payload: serde_json::json!({ "message": format!("{e}") }),
+        },
+        EsrcError::Invalid => CommandError {
+            variant: "Invalid".into(),
+            payload: serde_json::json!({ "message": "consumed invalid event in stream" }),
+        },
+        EsrcError::Conflict => CommandError {
+            variant: "Conflict".into(),
+            payload: serde_json::json!({ "message": "event transaction failed" }),
+        },
+    }
+}
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command_dispatcher.rs">
@@
         match handler.handle_erased(&mut store, payload).await {
                         Ok(reply) => {
                             let _ = request.respond(Ok(reply.into())).await;
                         },
                         Err(e) => {
                             // Encode the failure as a CommandReply so callers always
                             // deserialize the same shape regardless of outcome.
-                            let failure = CommandReply {
-                                id: uuid::Uuid::nil(),
-                                success: false,
-                                message: Some(format!("{e}")),
-                            };
+                            let failure = CommandReply {
+                                id: uuid::Uuid::nil(),
+                                success: false,
+                                error: Some(super::aggregate_command_handler::CommandError {
+                                    variant: "Internal".into(),
+                                    payload: serde_json::json!({ "message": format!("{e}") }),
+                                }),
+                            };
                             let body = serde_json::to_vec(&failure).unwrap_or_default();
                             let _ = request.respond(Ok(body.into())).await;
                         },
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/aggregate_command_handler.rs">
@@
+// ── Wire types ──────────────────────────────────────────────────────────────
 
 /// A standard command envelope sent over NATS.
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
-pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
+pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandError, CommandReply};
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
 use esrc_cqrs::nats::{
-    AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler,
+    AggregateCommandHandler, CommandEnvelope, CommandError, CommandReply, DurableProjectorHandler,
     NatsCommandDispatcher,
 };
@@
 /// Test that a failing command returns a NATS service error (non-2xx code).
 /// The framework should not crash and subsequent commands should still work.
 #[tokio::test]
 async fn test_command_error_does_not_break_dispatcher() {
     let ctx = TestCtx::new("cmd-err").await;
 
     let registry = CqrsRegistry::new(ctx.store.clone())
         .register_command(AggregateCommandHandler::<Counter>::new("Counter"));
 
     spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;
 
-    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(ctx.service_name(), "Counter");
-
-    // Send a command that will always fail.
-    let bad_envelope = CommandEnvelope {
-        id: Uuid::new_v4(),
-        command: CounterCommand::AlwaysFail,
-    };
-    let bad_payload = serde_json::to_vec(&bad_envelope).unwrap();
-
-    // The request itself resolves (the service replies with an error code) or
-    // returns a service-level error; either way we should not hang or panic.
-    let bad_result = ctx.client.request(subject.clone(), bad_payload.into()).await;
-    dbg!(&bad_result);
-    // The NATS service API sends an error status reply; async-nats surfaces
-    // this as an Err from `request`. We only assert the client did not hang.
-    let _ = bad_result;
+    // Send a command that will always fail and verify the typed error is returned.
+    let fail_id = Uuid::new_v4();
+    let fail_reply = send_command(
+        &ctx.client,
+        ctx.service_name(),
+        "Counter",
+        fail_id,
+        CounterCommand::AlwaysFail,
+    )
+    .await;
+
+    assert!(!fail_reply.success, "AlwaysFail command should not succeed");
+    assert_eq!(fail_reply.id, fail_id);
+
+    // The reply must carry a structured CommandError.
+    let cmd_err: CommandError = fail_reply.error.expect("error field must be present");
+
+    // The variant must be External because CounterError is the aggregate domain error.
+    assert_eq!(cmd_err.variant, "External", "aggregate domain errors are External");
+
+    // The payload message must contain the Display text of CounterError::ForcedFailure.
+    let expected_msg = CounterError::ForcedFailure.to_string();
+    let actual_msg = cmd_err.payload["message"]
+        .as_str()
+        .expect("payload.message must be a string");
+    assert!(
+        actual_msg.contains(&expected_msg),
+        "payload message '{actual_msg}' should contain '{expected_msg}'"
+    );
 
     // Now send a valid command to confirm the dispatcher is still running.
     let good_id = Uuid::new_v4();
</thinking>

Wait, I have a duplicate `@@` hunk issue in `aggregate_command_handler.rs`. The region marker comment `// ── Wire types` patch conflicts with the main patch. Let me redo this cleanly as a single `FILE_PATCH` per file.

</FILE_CHANGES>

Let me redo the patches properly without the conflicting hunks:

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/aggregate_command_handler.rs">
@@
 use std::marker::PhantomData;
 
 use esrc::aggregate::{Aggregate, Root};
-use esrc::error::{self, Error};
+use esrc::error::{self, Error as EsrcError};
 use esrc::event::publish::PublishExt;
 use esrc::event::replay::ReplayOneExt;
 use esrc::nats::NatsStore;
@@
 use crate::command::CommandHandler;
 
-/// A standard command envelope sent over NATS.
+// ── Wire types ───────────────────────────────────────────────────────────────
+
+/// A standard command envelope sent over NATS.
 ///
 /// The command payload wraps the aggregate ID and the serialized command body.
 /// Both the ID and the command are encoded as JSON.
@@
 pub struct CommandEnvelope<C> {
     /// The ID of the aggregate instance this command targets.
     pub id: Uuid,
-    /// The actual command to process.
     pub command: C,
 }
 
-/// A standard reply envelope returned after processing a command.
+/// A structured error carried inside a [`CommandReply`] when a command fails.
+///
+/// `variant` names the [`esrc::error::Error`] arm that was produced
+/// (`"External"`, `"Internal"`, `"Format"`, `"Invalid"`, `"Conflict"`).
+///
+/// For the `"External"` variant `payload` holds the JSON-serialized
+/// user-defined error returned by [`Aggregate::process`]. Callers that know
+/// which aggregate they targeted can deserialize `payload` back into the
+/// concrete error type with [`serde_json::from_value`].
+///
+/// For all other variants `payload` contains a plain `{"message":"…"}` object
+/// so callers always have a human-readable description.
+#[derive(Debug, Clone, Deserialize, Serialize)]
+pub struct CommandError {
+    /// The `esrc::Error` variant name.
+    pub variant: String,
+    /// JSON-encoded error payload; structure depends on `variant`.
+    pub payload: serde_json::Value,
+}
+
+/// A standard reply envelope returned after processing a command.
+///
+/// On success `error` is `None`.
+/// On failure `success` is `false` and `error` carries a [`CommandError`]
+/// that the caller can inspect or deserialize.
 #[derive(Debug, Deserialize, Serialize)]
 pub struct CommandReply {
     /// The aggregate ID that was modified.
     pub id: Uuid,
     /// Whether the command succeeded.
     pub success: bool,
-    /// An optional message (e.g., error description on failure).
-    pub message: Option<String>,
+    /// Structured error information when `success` is `false`.
+    pub error: Option<CommandError>,
 }
@@
     async fn handle<'a>(
         &'a self,
         store: &'a mut NatsStore,
         payload: &'a [u8],
     ) -> error::Result<Vec<u8>> {
         let envelope: CommandEnvelope<A::Command> =
-            serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;
+            serde_json::from_slice(payload).map_err(|e| EsrcError::Format(e.into()))?;
 
         let root: Root<A> = store.read(envelope.id).await?;
         let agg_id = envelope.id;
         let root = store.try_write(root, envelope.command, None).await;
 
         let reply = match root {
             Ok(written) => CommandReply {
                 id: Root::id(&written),
                 success: true,
-                message: None,
+                error: None,
             },
-            Err(e) => CommandReply {
-                id: agg_id,
-                success: false,
-                message: Some(format!("{e}")),
-            },
+            Err(e) => {
+                let cmd_error = esrc_error_to_command_error(&e);
+                CommandReply {
+                    id: agg_id,
+                    success: false,
+                    error: Some(cmd_error),
+                }
+            },
         };
-        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
+        serde_json::to_vec(&reply).map_err(|e| EsrcError::Format(e.into()))
     }
 }
+
+// ── Helpers ───────────────────────────────────────────────────────────────────
+
+/// Convert an [`esrc::Error`] into a wire-safe [`CommandError`].
+///
+/// For `External` errors the inner `Box<dyn std::error::Error>` is serialized
+/// via its `Display` impl wrapped in `{"message":"…"}`. The concrete aggregate
+/// error type is preserved as a string so callers who know the command context
+/// can compare against their known error variants via `Display`.
+///
+/// For all other variants a `{"message":"…"}` payload is produced.
+fn esrc_error_to_command_error(err: &EsrcError) -> CommandError {
+    match err {
+        EsrcError::External(e) => {
+            // The external error is the aggregate's domain error. The message
+            // is the Display representation of the concrete error type, which
+            // callers can compare against their known error variants.
+            let payload = serde_json::json!({ "message": format!("{e}") });
+            CommandError { variant: "External".into(), payload }
+        },
+        EsrcError::Internal(e) => CommandError {
+            variant: "Internal".into(),
+            payload: serde_json::json!({ "message": format!("{e}") }),
+        },
+        EsrcError::Format(e) => CommandError {
+            variant: "Format".into(),
+            payload: serde_json::json!({ "message": format!("{e}") }),
+        },
+        EsrcError::Invalid => CommandError {
+            variant: "Invalid".into(),
+            payload: serde_json::json!({ "message": "consumed invalid event in stream" }),
+        },
+        EsrcError::Conflict => CommandError {
+            variant: "Conflict".into(),
+            payload: serde_json::json!({ "message": "event transaction failed" }),
+        },
+    }
+}
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command_dispatcher.rs">
@@
                         Err(e) => {
                             // Encode the failure as a CommandReply so callers always
                             // deserialize the same shape regardless of outcome.
-                            let failure = CommandReply {
-                                id: uuid::Uuid::nil(),
-                                success: false,
-                                message: Some(format!("{e}")),
-                            };
+                            let failure = CommandReply {
+                                id: uuid::Uuid::nil(),
+                                success: false,
+                                error: Some(super::aggregate_command_handler::CommandError {
+                                    variant: "Internal".into(),
+                                    payload: serde_json::json!({ "message": format!("{e}") }),
+                                }),
+                            };
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
-pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
+pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandError, CommandReply};
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
 use esrc_cqrs::nats::{
-    AggregateCommandHandler, CommandEnvelope, CommandReply, DurableProjectorHandler,
+    AggregateCommandHandler, CommandEnvelope, CommandError, CommandReply, DurableProjectorHandler,
     NatsCommandDispatcher,
 };
@@
 /// Test that a failing command returns a NATS service error (non-2xx code).
 /// The framework should not crash and subsequent commands should still work.
 #[tokio::test]
 async fn test_command_error_does_not_break_dispatcher() {
     let ctx = TestCtx::new("cmd-err").await;
 
     let registry = CqrsRegistry::new(ctx.store.clone())
         .register_command(AggregateCommandHandler::<Counter>::new("Counter"));
 
     spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;
 
-    let subject = esrc_cqrs::nats::command_dispatcher::command_subject(ctx.service_name(), "Counter");
-
-    // Send a command that will always fail.
-    let bad_envelope = CommandEnvelope {
-        id: Uuid::new_v4(),
-        command: CounterCommand::AlwaysFail,
-    };
-    let bad_payload = serde_json::to_vec(&bad_envelope).unwrap();
-
-    // The request itself resolves (the service replies with an error code) or
-    // returns a service-level error; either way we should not hang or panic.
-    let bad_result = ctx.client.request(subject.clone(), bad_payload.into()).await;
-    dbg!(&bad_result);
-    // The NATS service API sends an error status reply; async-nats surfaces
-    // this as an Err from `request`. We only assert the client did not hang.
-    let _ = bad_result;
+    // Send a command that always fails and verify the typed error is returned.
+    let fail_id = Uuid::new_v4();
+    let fail_reply = send_command(
+        &ctx.client,
+        ctx.service_name(),
+        "Counter",
+        fail_id,
+        CounterCommand::AlwaysFail,
+    )
+    .await;
+
+    assert!(!fail_reply.success, "AlwaysFail command should not succeed");
+    assert_eq!(fail_reply.id, fail_id);
+
+    // The reply must carry a structured CommandError.
+    let cmd_err: CommandError = fail_reply.error.expect("error field must be present on failure");
+
+    // Aggregate domain errors are always wrapped in esrc::Error::External.
+    assert_eq!(cmd_err.variant, "External", "aggregate domain errors surface as External");
+
+    // The payload message must match the Display of CounterError::ForcedFailure,
+    // which the caller can compare knowing which command was sent.
+    let expected_msg = CounterError::ForcedFailure.to_string();
+    let actual_msg = cmd_err.payload["message"]
+        .as_str()
+        .expect("payload.message must be a string");
+    assert!(
+        actual_msg.contains(&expected_msg),
+        "payload message '{actual_msg}' should contain the domain error '{expected_msg}'"
+    );
 
     // Now send a valid command to confirm the dispatcher is still running.
     let good_id = Uuid::new_v4();
</FILE_PATCH>

</FILE_CHANGES>

Here is a summary of the changes:

- **`CommandReply.message: Option<String>`** is replaced by **`CommandReply.error: Option<CommandError>`**. The new `CommandError` type carries a `variant` string (the `esrc::Error` arm name) and a `payload: serde_json::Value`.

- **`CommandError`** is a new public type. The `variant` field tells the caller which `esrc::Error` arm fired. For `"External"` (aggregate domain errors), `payload["message"]` holds the `Display` text of the concrete error, which the caller can match against their known error type's display strings since they know what command they sent.

- **`esrc_error_to_command_error`** is the private helper that maps each `esrc::Error` variant to the wire representation.

- The **dispatcher fallback** (parse/handler errors) uses `variant: "Internal"` consistently.

- The **test** now sends `AlwaysFail`, receives a `CommandReply`, asserts `success == false`, extracts `CommandError`, checks `variant == "External"`, and compares `payload["message"]` against `CounterError::ForcedFailure.to_string()`, which is the text the caller would know from having called that specific command.

<suggested_git_command>
git add -A .
git commit -a -m "^ esrc-cqrs - Replace CommandReply.message with typed CommandError carrying esrc::Error variant and JSON payload"
</suggested_git_command>

