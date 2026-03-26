====
>   Info: Duration: 22s | ~$0.1518 | Prompt Tokens: 21,324 (cached: 0, cache_creation: 0) | Completion Tokens: 1,809 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 10 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the plan, there's an active step (Rename ConsumerName to ComponentName) that's already been implemented. The user says "do next step," so I need to:

1. Move the active step to done
2. Activate the topmost todo step ("Create src/query.rs with Query, QueryHandler, QueryTransport, and QuerySpec")
3. Implement it

Let me review the step requirements and implement accordingly.

<FILE_CHANGES>

<FILE_NEW file_path="src/query.rs">
//! Traits and types for declaring and handling queries against read models.

use crate::event_modeling::ComponentName;

/// A query that can be executed against a read model.
///
/// Defines the association between a query enum, its target read model,
/// and the response type. Kept minimal with no serde bounds; serialization
/// requirements are pushed to transport-specific traits.
pub trait Query: Send {
    /// The read model this query targets.
    type ReadModel: Send;
    /// The response type returned by executing this query.
    type Response: Send;
}

/// Handles queries for a specific read model.
///
/// Includes a built-in `get_by_id` method that every handler must implement,
/// plus a `handle` method for custom query enum variants. Uses `esrc::error::Error`
/// with `External` for domain/persistence errors (matching the command handler model).
///
/// Does not require `Clone`; sharing via `Arc` is expected.
#[trait_variant::make(Send)]
pub trait QueryHandler: Send + Sync {
    /// The query enum this handler responds to.
    type Query: Query;
    /// The identifier type used to look up individual read model instances.
    type Id: Send + Sync;

    /// Get a single read model instance by its identifier.
    ///
    /// Returns `Ok(None)` when the read model is not found.
    async fn get_by_id(
        &self,
        id: Self::Id,
    ) -> crate::error::Result<Option<<Self::Query as Query>::ReadModel>>;

    /// Execute a custom query.
    async fn handle(
        &self,
        query: Self::Query,
    ) -> crate::error::Result<<Self::Query as Query>::Response>;
}

/// Transport mechanism for exposing queries remotely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryTransport {
    /// Expose queries via NATS request-reply.
    NatsRequestReply,
}

/// A normalized query declaration that can later be executed by infrastructure.
///
/// Analogous to `ConsumerSpec`, declares metadata about how queries are exposed.
#[derive(Clone, Debug)]
pub struct QuerySpec<H> {
    name: ComponentName,
    transport: QueryTransport,
    handler: H,
}

impl<H> QuerySpec<H> {
    /// Create a new query specification.
    pub fn new(name: ComponentName, transport: QueryTransport, handler: H) -> Self {
        Self {
            name,
            transport,
            handler,
        }
    }

    /// Returns the structured component name for this query.
    pub fn name(&self) -> &ComponentName {
        &self.name
    }

    /// Returns the configured transport for this query.
    pub fn transport(&self) -> QueryTransport {
        self.transport
    }

    /// Returns a reference to the configured query handler.
    pub fn handler(&self) -> &H {
        &self.handler
    }

    /// Returns a mutable reference to the configured query handler.
    pub fn handler_mut(&mut self) -> &mut H {
        &mut self.handler
    }

    /// Consumes the specification and returns the configured query handler.
    pub fn into_handler(self) -> H {
        self.handler
    }

    /// Override the transport for this query.
    pub fn with_transport(mut self, transport: QueryTransport) -> Self {
        self.transport = transport;
        self
    }
}
</FILE_NEW>

<FILE_PATCH file_path="src/lib.rs">
@@
 pub mod project;
+/// Traits and types for declaring and handling queries against read models.
+pub mod query;
 /// (De)Serialize types with extra version information for upcasting.
</FILE_PATCH>

<FILE_NEW file_path="_workbench/queries/plan-3-done-steps.md">
# Plan 3 - Done Steps

## Step - Rename ConsumerName to ComponentName in event_modeling.rs
      status: done
time-created: 2026-03-26 14:58:37
   time-done: 2026-03-26 15:04:08

- Renamed the `ConsumerName` struct to `ComponentName` in `src/event_modeling.rs`.
- Renamed the `consumer` field to `component` (and updated the associated `consumer()` method to `component()`).
- Updated `durable_name()` and `slice_path()` to use the new field name.
- Updated all references in `ConsumerSpec`, `Automation`, `ReadModel` builders within the same file.
- Kept `ConsumerRole`, `ConsumerSpec`, `Automation`, and `ReadModel` names unchanged.
- Updated all usages across the codebase: example files that reference `ConsumerName` or the `consumer` field/method.
- `src/nats.rs` and `src/nats/command_service.rs` do not directly reference `ConsumerName` by import (they use `ConsumerSpec` which internally holds `ComponentName`), so no changes needed there.
</FILE_NEW>

<FILE_NEW file_path="_workbench/queries/plan-2-active-step.md">
# Plan 2 - Active Step

## Step - Create src/query.rs with Query, QueryHandler, QueryTransport, and QuerySpec
      status: active
time-created: 2026-03-26 14:58:37
time-current: 2026-03-26 15:08:42

- Created `src/query.rs` as a single file containing:

- `Query` trait:
  - `type ReadModel: Send`
  - `type Response: Send`
  - No serde bounds.

- `QueryHandler` trait (using `#[trait_variant::make(Send)]`):
  - `type Query: Query`
  - `type Id: Send + Sync`
  - `async fn get_by_id(&self, id: Self::Id) -> crate::error::Result<Option<<Self::Query as Query>::ReadModel>>`
  - `async fn handle(&self, query: Self::Query) -> crate::error::Result<<Self::Query as Query>::Response>`
  - Requires `Send + Sync`, no `Clone`.

- `QueryTransport` enum:
  - `NatsRequestReply` variant (only one for now, extensible).

- `QuerySpec` struct:
  - Holds a `ComponentName` (from `event_modeling`), a `QueryTransport`, and the `QueryHandler` instance.
  - Constructor and accessor methods following the `ConsumerSpec` pattern.

- Registered the module in `src/lib.rs` with a doc comment.
</FILE_NEW>

</FILE_CHANGES>

<suggested_git_command>
git add -A . && git commit -a -m "+ query - Add Query, QueryHandler, QueryTransport, and QuerySpec in src/query.rs"
</suggested_git_command>

