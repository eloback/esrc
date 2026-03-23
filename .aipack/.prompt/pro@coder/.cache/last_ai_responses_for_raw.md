====
>   Info: Duration: 45s | ~$0.1261 | Prompt Tokens: 25,592 (cached: 0, cache_creation: 0) | Completion Tokens: 3,288 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 13 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll analyze the current boilerplate and design a plan to improve the developer experience for both commands and queries in the `esrc-cqrs` framework.

## Analysis of Current Boilerplate

For a command, the caller must:
1. Construct a `CommandEnvelope` manually
2. Serialize it with `serde_json::to_vec`
3. Build the subject string via `command_dispatcher::command_subject`
4. Call `nats_client.request(...)`
5. Deserialize the `CommandReply`
6. Check `reply.success` and branch

For a query, the same steps apply with `QueryEnvelope` and `QueryReply`.

## Plan

### Phase 1: `NatsClient` wrapper (no proc macro needed yet)

Create a high-level `EsrcNatsClient` (or `CqrsClient`) struct that wraps an `async_nats::Client` and exposes ergonomic `send_command` and `send_query` methods. This eliminates all the boilerplate in a single call with typed generics, no derive macros needed.

```rust
// Target DX
let reply = client.send_command("clt-service", "Escrituracao", batch_id, cmd).await?;
let state: CounterState = client.send_query("clt-service", "Counter.GetState", id).await?;
```

### Phase 2: `esrc_cqrs_macros` proc macro crate (optional enhancement)

A `#[cqrs_command]` and `#[cqrs_query]` attribute macro on handler structs that auto-derives the service/handler name constants and generates a typed `dispatch` associated function, so callers never construct envelopes manually.

### Immediate deliverable: Phase 1

This gives the biggest DX win with zero proc macros and zero new dependencies. I'll implement:

1. `crates/esrc-cqrs/src/nats/client.rs` - the `CqrsClient` struct
2. Export it from `mod.rs`
3. Add a usage example in the cafe example

Let me implement Phase 1 now:

<FILE_CHANGES>

<FILE_NEW file_path="crates/esrc-cqrs/src/nats/client.rs">
use async_nats::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use esrc::error::{self, Error};

use super::aggregate_command_handler::{CommandEnvelope, CommandReply};
use super::command_dispatcher::command_subject;
use super::query_dispatcher::{QueryEnvelope, QueryReply, query_subject};

/// High-level CQRS client that removes boilerplate from command and query dispatch.
///
/// `CqrsClient` wraps an `async_nats::Client` and handles envelope construction,
/// serialization, subject building, and reply deserialization internally. Callers
/// only supply the service name, handler name, aggregate ID, and payload.
///
/// # Command dispatch
///
/// ```rust,ignore
/// let reply = client
///     .send_command("my-service", "MyAggregate", aggregate_id, my_command)
///     .await?;
/// ```
///
/// # Query dispatch
///
/// ```rust,ignore
/// let state: MyState = client
///     .send_query("my-service", "MyAggregate.GetState", aggregate_id)
///     .await?;
/// ```
#[derive(Clone, Debug)]
pub struct CqrsClient {
    inner: Client,
}

impl CqrsClient {
    /// Create a new `CqrsClient` wrapping the given NATS client.
    pub fn new(client: Client) -> Self {
        Self { inner: client }
    }

    /// Return a reference to the underlying `async_nats::Client`.
    pub fn inner(&self) -> &Client {
        &self.inner
    }

    /// Send a command to a handler and return the raw [`CommandReply`].
    ///
    /// The envelope is constructed and serialized internally. The subject is
    /// built from `service_name` and `handler_name` using [`command_subject`].
    ///
    /// # Errors
    ///
    /// Returns an [`esrc::error::Error::Internal`] if the NATS request fails or
    /// the reply cannot be deserialized. A successful return does not imply
    /// `reply.success == true`; the caller should inspect [`CommandReply::success`].
    pub async fn send_command<C>(
        &self,
        service_name: &str,
        handler_name: &str,
        id: Uuid,
        command: C,
    ) -> error::Result<CommandReply>
    where
        C: Serialize,
    {
        let envelope = CommandEnvelope { id, command };
        let payload =
            serde_json::to_vec(&envelope).map_err(|e| Error::Format(e.into()))?;
        let subject = command_subject(service_name, handler_name);

        let msg = self
            .inner
            .request(subject, payload.into())
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        serde_json::from_slice::<CommandReply>(&msg.payload)
            .map_err(|e| Error::Format(e.into()))
    }

    /// Send a command and return `Ok(reply.id)` on success, or convert the
    /// [`CommandReply`] error into an [`esrc::error::Error`] on failure.
    ///
    /// This is a convenience wrapper around [`send_command`] for callers that
    /// want to propagate command failures as `Result::Err` rather than
    /// inspecting the reply manually.
    ///
    /// [`send_command`]: CqrsClient::send_command
    pub async fn dispatch_command<C>(
        &self,
        service_name: &str,
        handler_name: &str,
        id: Uuid,
        command: C,
    ) -> error::Result<Uuid>
    where
        C: Serialize,
    {
        let reply = self.send_command(service_name, handler_name, id, command).await?;
        if reply.success {
            Ok(reply.id)
        } else {
            let msg = reply
                .error
                .as_ref()
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "command failed".to_string());
            Err(Error::Internal(msg.into()))
        }
    }

    /// Send a query to a handler and return the raw [`QueryReply`].
    ///
    /// The envelope is constructed and serialized internally. The subject is
    /// built from `service_name` and `handler_name` using [`query_subject`].
    ///
    /// # Errors
    ///
    /// Returns an [`esrc::error::Error::Internal`] if the NATS request fails or
    /// the reply cannot be deserialized. A successful return does not imply
    /// `reply.success == true`; the caller should inspect [`QueryReply::success`].
    pub async fn send_query(
        &self,
        service_name: &str,
        handler_name: &str,
        id: Uuid,
    ) -> error::Result<QueryReply>
    where
    {
        let envelope = QueryEnvelope { id };
        let payload =
            serde_json::to_vec(&envelope).map_err(|e| Error::Format(e.into()))?;
        let subject = query_subject(service_name, handler_name);

        let msg = self
            .inner
            .request(subject, payload.into())
            .await
            .map_err(|e| Error::Internal(e.into()))?;

        serde_json::from_slice::<QueryReply>(&msg.payload)
            .map_err(|e| Error::Format(e.into()))
    }

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
            let msg = reply
                .error
                .as_ref()
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "query failed".to_string());
            return Err(Error::Internal(msg.into()));
        }
        let data = reply.data.ok_or_else(|| {
            Error::Internal("query succeeded but returned no data".into())
        })?;
        serde_json::from_value::<T>(data).map_err(|e| Error::Format(e.into()))
    }
}

impl From<Client> for CqrsClient {
    fn from(client: Client) -> Self {
        Self::new(client)
    }
}
</FILE_NEW>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
 pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
+pub use client::CqrsClient;
 pub use command_dispatcher::NatsCommandDispatcher;
@@
 pub use query_dispatcher::{NatsQueryDispatcher, QueryEnvelope, QueryReply};
 
 /// Aggregate command handler wiring: maps a typed command to an aggregate and writes events.
 mod aggregate_command_handler;
+/// High-level CQRS client for ergonomic command and query dispatch.
+mod client;
 /// Durable projector handler wiring: maps a projector to a durable JetStream consumer.
 mod durable_projector_handler;
</FILE_PATCH>

<FILE_PATCH file_path="crates/esrc-cqrs/src/lib.rs">
@@
 pub use command::CommandHandler;
 pub use error::Error;
+#[cfg(feature = "nats")]
+pub use nats::CqrsClient;
 pub use projector::ProjectorHandler;
</FILE_PATCH>

</FILE_CHANGES>

Now here is the improved DX the plan delivers, shown as a before/after comparison:

**Before (current boilerplate):**
```rust
let command_envelope = CommandEnvelope { id: body.batch_id, command: cmd };
let payload = serde_json::to_vec(&command_envelope).expect("serialize");
let subject = esrc_cqrs::nats::command_dispatcher::command_subject("clt-service", "Escrituracao");
let reply = state.nats_client.request(subject, payload.into()).await.unwrap();
let reply: CommandReply = serde_json::from_slice(&reply.payload).unwrap();
if !reply.success { return Err(ErrorResponses::from(reply.error.unwrap())); }
```

**After (with `CqrsClient`):**
```rust
// Option A: get typed result directly, failure becomes Err
let id = state.cqrs.dispatch_command("clt-service", "Escrituracao", body.batch_id, cmd).await?;

// Option B: get the raw reply if you need to inspect the error type
let reply = state.cqrs.send_command("clt-service", "Escrituracao", body.batch_id, cmd).await?;

// Query - typed deserialization in one call
let state: MyState = cqrs.dispatch_query("clt-service", "MyAggregate.GetState", id).await?;
```

**Two-method design per verb:**

- `send_command` / `send_query`: returns the raw reply envelope (gives access to the `CommandReply::error` typed field for downcasting aggregate errors).
- `dispatch_command` / `dispatch_query`: converts `success=false` into `Err` automatically, for the common case where you just want to propagate failures.

**Phase 2 (proc macro) would add**, when warranted by usage patterns: a `#[cqrs_handler(service = "...", name = "...")]` attribute that generates an associated `dispatch` method directly on the command type, so the service/handler name strings are co-located with the type definition rather than scattered across call sites.

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs/nats - Add CqrsClient for ergonomic command and query dispatch"
</suggested_git_command>

