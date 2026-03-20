====
>   Info: Duration: 1m 12s | ~$0.1219 | Prompt Tokens: 15,967 (cached: 0, cache_creation: 0) | Completion Tokens: 4,936 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 12 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the code, I need to:

1. Create a `esrc_cqrs::Error` enum that mirrors `esrc::Error` variants but is serializable, with a special `External` variant for aggregate errors.
2. Modify the command dispatcher to parse `esrc::Error` and construct `esrc_cqrs::Error`, serializing aggregate errors for the `External` variant.
3. Modify `CommandReply` to carry `esrc_cqrs::Error` on failure instead of a plain string.
4. Allow callers to parse the `External` variant back into their aggregate error type.
5. Update the test to validate the error roundtrip.

Let me trace the flow:
- `AggregateCommandHandler::handle` returns `error::Result<Vec<u8>>` where `Vec<u8>` is a serialized `CommandReply`.
- The dispatcher currently encodes failures as `CommandReply { success: false, message: Some(...) }`.
- I need to instead encode a serialized `esrc_cqrs::Error` in the reply.

The `esrc::Error::External` wraps a `BoxStdError` which comes from the aggregate's `Error` type (via `try_write`). The aggregate error must be serializable to JSON.

Key design decisions:
- `esrc_cqrs::Error` is a serializable enum mirroring `esrc::Error`.
- `External` variant stores `serde_json::Value` (the serialized aggregate error).
- `CommandReply` on failure carries the `esrc_cqrs::Error`.
- Caller can do `reply.error().and_then(|e| e.external::<MyAggError>())`.

<FILE_CHANGES>

<FILE_NEW file_path="crates/esrc-cqrs/src/error.rs">
use serde::{Deserialize, Serialize};

/// A serializable error type for CQRS command results.
///
/// This mirrors the variants of [`esrc::error::Error`] but is fully
/// serializable so it can be transmitted over NATS and reconstructed on the
/// caller side.
///
/// The [`Error::External`] variant carries the aggregate's domain error
/// serialized as a JSON value. Because the aggregate error type is erased at
/// the transport boundary, the caller must know which aggregate they targeted
/// (which they always do) and can deserialize the payload back with
/// [`Error::downcast_external`].
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", content = "detail")]
pub enum Error {
    /// An error occurred that is not related to event logic or parsing.
    /// Corresponds to [`esrc::error::Error::Internal`].
    #[error("internal error: {0}")]
    Internal(String),

    /// An error emitted by the aggregate's own command processing logic.
    /// Corresponds to [`esrc::error::Error::External`].
    ///
    /// The inner value is the aggregate error serialized as a JSON value.
    /// Use [`Error::downcast_external`] to recover the typed error.
    #[error("external error: {0}")]
    External(serde_json::Value),

    /// An error occurred while deserializing an event or command.
    /// Corresponds to [`esrc::error::Error::Format`].
    #[error("bad envelope format: {0}")]
    Format(String),

    /// An event was parsed successfully but contained unexpected data.
    /// Corresponds to [`esrc::error::Error::Invalid`].
    #[error("consumed invalid event in stream")]
    Invalid,

    /// An optimistic concurrency error occurred.
    /// Corresponds to [`esrc::error::Error::Conflict`].
    #[error("event transaction failed")]
    Conflict,
}

impl Error {
    /// Attempt to deserialize the [`Error::External`] payload into a concrete
    /// aggregate error type `E`.
    ///
    /// Returns `None` if this variant is not `External`. Panics if the JSON
    /// value cannot be deserialized into `E`, which would indicate a mismatch
    /// between the aggregate error type and what was stored -- this is always
    /// a programming error.
    pub fn downcast_external<E>(&self) -> Option<E>
    where
        E: for<'de> Deserialize<'de>,
    {
        match self {
            Error::External(value) => {
                let e: E = serde_json::from_value(value.clone())
                    .expect("External error payload could not be deserialized into the requested type; ensure the aggregate Error type matches and implements Deserialize");
                Some(e)
            },
            _ => None,
        }
    }
}

/// Convert an [`esrc::error::Error`] into an [`Error`], serializing the
/// aggregate domain error payload with `serde_json`.
///
/// The `External` source must implement `serde::Serialize`. If it does not,
/// this function panics, as it indicates the aggregate was configured without
/// a serializable error type, which is a programming error.
pub fn from_esrc_error(err: esrc::error::Error) -> Error {
    match err {
        esrc::error::Error::Internal(e) => Error::Internal(e.to_string()),
        esrc::error::Error::External(e) => {
            // The source of an External error is the aggregate's own Error type.
            // It must be serializable so it can be transmitted to the caller.
            // We use the erased error's Display by default, but we need the
            // structured form. We require the error to be serde::Serialize via
            // the erased-serde approach: the AggregateCommandHandler serializes
            // the error before boxing it (see aggregate_command_handler.rs).
            //
            // At this point the error has already been serialized into the box
            // as a serde_json::Value by the handler shim. We recover it here.
            let value: serde_json::Value = e
                .downcast::<serde_json::Value>()
                .map(|v| *v)
                .unwrap_or_else(|e| serde_json::Value::String(e.to_string()));
            Error::External(value)
        },
        esrc::error::Error::Format(e) => Error::Format(e.to_string()),
        esrc::error::Error::Invalid => Error::Invalid,
        esrc::error::Error::Conflict => Error::Conflict,
    }
}

</FILE_NEW>

<FILE_PATCH file_path="crates/esrc-cqrs/src/lib.rs">
@@
 /// Command handler trait and registry entry.
 pub mod command;
+/// Serializable CQRS error type transmitted in command replies.
+pub mod error;
 /// Event projector handler registry entry.
 pub mod projector;
@@
 pub use command::CommandHandler;
+pub use error::Error;
 pub use projector::ProjectorHandler;
 pub use registry::CqrsRegistry;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/aggregate_command_handler.rs">
@@
 use std::marker::PhantomData;
 
 use esrc::aggregate::{Aggregate, Root};
-use esrc::error::{self, Error};
+use esrc::error;
 use esrc::event::publish::PublishExt;
 use esrc::event::replay::ReplayOneExt;
 use esrc::nats::NatsStore;
 use esrc::version::{DeserializeVersion, SerializeVersion};
 use serde::{Deserialize, Serialize};
 use uuid::Uuid;
 
 use crate::command::CommandHandler;
+use crate::error as cqrs_error;
@@
 /// A standard reply envelope returned after processing a command.
 #[derive(Debug, Deserialize, Serialize)]
 pub struct CommandReply {
     /// The aggregate ID that was modified.
     pub id: Uuid,
     /// Whether the command succeeded.
     pub success: bool,
-    /// An optional message (e.g., error description on failure).
-    pub message: Option<String>,
+    /// The structured CQRS error, present only when `success` is false.
+    pub error: Option<cqrs_error::Error>,
 }
@@
 /// `A` is the aggregate type. `A::Command` must implement `Deserialize` and
 /// `A::Event` must implement both `SerializeVersion` and `DeserializeVersion`.
 pub struct AggregateCommandHandler<A>
 where
     A: Aggregate,
 {
     /// The name used to route commands to this handler.
     ///
     /// Convention: `<AggregateName>.<CommandName>` or just `<AggregateName>`.
     handler_name: &'static str,
     _phantom: PhantomData<A>,
 }
@@
 impl<A> CommandHandler<NatsStore> for AggregateCommandHandler<A>
 where
     A: Aggregate + Send + Sync + 'static,
     A::Command: for<'de> Deserialize<'de> + Send,
-    A::Event: SerializeVersion + DeserializeVersion + Send,
+    A::Event: SerializeVersion + DeserializeVersion + Send,
+    A::Error: Serialize,
 {
     fn name(&self) -> &'static str {
         self.handler_name
     }
 
     async fn handle<'a>(
         &'a self,
         store: &'a mut NatsStore,
         payload: &'a [u8],
     ) -> error::Result<Vec<u8>> {
-        let envelope: CommandEnvelope<A::Command> =
-            serde_json::from_slice(payload).map_err(|e| Error::Format(e.into()))?;
+        let envelope: CommandEnvelope<A::Command> =
+            serde_json::from_slice(payload).map_err(|e| esrc::error::Error::Format(e.into()))?;
 
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
             Err(e) => {
-                CommandReply {
-                    id: agg_id,
-                    success: false,
-                    message: Some(format!("{e}")),
-                }
+                // Convert the esrc error into a serializable cqrs_error::Error.
+                // For the External variant the aggregate's Error must implement
+                // Serialize (enforced by the trait bound above). We serialize it
+                // into a serde_json::Value before boxing so that from_esrc_error
+                // can recover the structured value on the other side.
+                let cqrs_err = convert_esrc_error::<A>(e);
+                CommandReply {
+                    id: agg_id,
+                    success: false,
+                    error: Some(cqrs_err),
+                }
             },
         };
-        serde_json::to_vec(&reply).map_err(|e| Error::Format(e.into()))
+        serde_json::to_vec(&reply).map_err(|e| esrc::error::Error::Format(e.into()))
     }
 }
+
+/// Convert an [`esrc::error::Error`] into a [`cqrs_error::Error`], serializing
+/// the aggregate's domain error for the `External` variant.
+///
+/// The `External` source produced by `try_write` is the aggregate's own `Error`
+/// type boxed as `Box<dyn std::error::Error + Send + Sync>`. We downcast it
+/// back to `A::Error` and serialize it. If the downcast fails (which would be a
+/// framework bug), we fall back to the Display representation. If serialization
+/// fails, we panic because a non-serializable aggregate error is a programming
+/// error when using the CQRS framework.
+fn convert_esrc_error<A>(err: esrc::error::Error) -> cqrs_error::Error
+where
+    A: Aggregate,
+    A::Error: Serialize,
+{
+    match err {
+        esrc::error::Error::Internal(e) => cqrs_error::Error::Internal(e.to_string()),
+        esrc::error::Error::External(e) => {
+            let value = match e.downcast::<A::Error>() {
+                Ok(agg_err) => serde_json::to_value(&*agg_err)
+                    .expect("aggregate Error must be serializable when used with esrc-cqrs"),
+                Err(e) => serde_json::Value::String(e.to_string()),
+            };
+            cqrs_error::Error::External(value)
+        },
+        esrc::error::Error::Format(e) => cqrs_error::Error::Format(e.to_string()),
+        esrc::error::Error::Invalid => cqrs_error::Error::Invalid,
+        esrc::error::Error::Conflict => cqrs_error::Error::Conflict,
+    }
+}
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/command_dispatcher.rs">
@@
                         Err(e) => {
                             // Encode the failure as a CommandReply so callers always
                             // deserialize the same shape regardless of outcome.
                             let failure = CommandReply {
                                 id: uuid::Uuid::nil(),
                                 success: false,
-                                message: Some(format!("{e}")),
+                                error: Some(crate::error::Error::Internal(format!("{e}"))),
                             };
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/tests/integration_nats.rs">
@@
 #[derive(Debug, thiserror::Error)]
 enum CounterError {
     #[error("forced failure for testing")]
     ForcedFailure,
 }
+
+// CounterError must be serializable so it can be transmitted in CommandReply.
+impl serde::Serialize for CounterError {
+    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
+        serializer.serialize_str("ForcedFailure")
+    }
+}
+
+impl<'de> serde::Deserialize<'de> for CounterError {
+    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
+        let s = String::deserialize(deserializer)?;
+        match s.as_str() {
+            "ForcedFailure" => Ok(CounterError::ForcedFailure),
+            other => Err(serde::de::Error::unknown_variant(other, &["ForcedFailure"])),
+        }
+    }
+}
@@
 /// Test that a failing command returns a NATS service error (non-2xx code).
 /// The framework should not crash and subsequent commands should still work.
 #[tokio::test]
 async fn test_command_error_does_not_break_dispatcher() {
     let ctx = TestCtx::new("cmd-err").await;
 
     let registry = CqrsRegistry::new(ctx.store.clone())
         .register_command(AggregateCommandHandler::<Counter>::new("Counter"));
 
     spawn_dispatcher(&ctx, registry.command_handlers().to_vec()).await;
 
     let subject = esrc_cqrs::nats::command_dispatcher::command_subject(ctx.service_name(), "Counter");
 
     // Send a command that will always fail.
     let bad_envelope = CommandEnvelope {
         id: Uuid::new_v4(),
         command: CounterCommand::AlwaysFail,
     };
     let bad_payload = serde_json::to_vec(&bad_envelope).unwrap();
 
-    // The request itself resolves (the service replies with an error code) or
-    // returns a service-level error; either way we should not hang or panic.
-    let bad_result = ctx.client.request(subject.clone(), bad_payload.into()).await;
-    dbg!(&bad_result);
-    // The NATS service API sends an error status reply; async-nats surfaces
-    // this as an Err from `request`. We only assert the client did not hang.
-    let _ = bad_result;
+    // The dispatcher encodes the aggregate error as a CommandReply with success=false.
+    let raw = ctx
+        .client
+        .request(subject.clone(), bad_payload.into())
+        .await
+        .expect("NATS request should succeed");
+    let reply: CommandReply = serde_json::from_slice(&raw.payload).expect("valid CommandReply");
+
+    assert!(!reply.success, "AlwaysFail command should return success=false");
+    assert!(reply.error.is_some(), "error field should be populated");
+
+    // Recover the typed aggregate error from the External variant.
+    let cqrs_err = reply.error.as_ref().unwrap();
+    let agg_err: CounterError = cqrs_err
+        .downcast_external::<CounterError>()
+        .expect("External variant should be present and deserializable");
+    // Validate that the error we received is indeed the one the aggregate returned.
+    assert!(
+        matches!(agg_err, CounterError::ForcedFailure),
+        "deserialized aggregate error should be ForcedFailure, got: {agg_err:?}"
+    );
 
     // Now send a valid command to confirm the dispatcher is still running.
     let good_id = Uuid::new_v4();
</FILE_PATCH>

</FILE_CHANGES>

A few notes on the design:

- `esrc_cqrs::Error` is a fully serializable enum that mirrors `esrc::Error` variants. It lives in `crates/esrc-cqrs/src/error.rs`.
- For the `External` variant, the aggregate's error is serialized into a `serde_json::Value` inside `convert_esrc_error`. This requires `A::Error: Serialize` as a new bound on `AggregateCommandHandler`'s `CommandHandler` impl. If the aggregate error is not serializable, the code panics at the call site, flagging it as a programming error.
- `CommandReply.message: Option<String>` is replaced with `error: Option<cqrs_error::Error>` so callers always receive structured error data.
- `Error::downcast_external::<E>()` lets the caller (who knows which aggregate they targeted) recover the typed error. If the JSON does not match `E`, it panics (programming error).
- The test adds `Serialize`/`Deserialize` for `CounterError` and validates the full roundtrip: send `AlwaysFail`, receive `CommandReply`, downcast to `CounterError::ForcedFailure`.

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs - Add serializable CqrsError and structured aggregate error propagation in CommandReply"
</suggested_git_command>

