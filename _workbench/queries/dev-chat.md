# Dev Chat

Add a new `## Request: _user_ask_title_concise_` with the answer below (concise title). Use markdown sub-headings for sub sections. Keep this top instruction in this file.

## Request: Query Framework Design for Read Models

### Context

The `esrc` crate already has:

- `View` trait for lightweight read models built from event streams.
- `Project` trait for projecting events into read models with side effects.
- `ConsumerSpec`, `ReadModel`, `Automation` in `event_modeling.rs` for declaring consumer specifications.

The goal is to extend the framework with a **Query** layer that lets developers declare how data is fetched from read models.

### Core Idea

Every read model should be queryable. The query layer provides a structured way to:

- Declare what queries a read model supports.
- Ensure every read model has an identifier and a default "get by ID" query.
- Allow additional queries (by multiple fields, filtered lists, aggregations, etc.).
- Let the developer choose how queries are exposed (similar to how `ConsumerSpec` lets them choose execution policy).

### Design Sketch

#### Read Model Identity

- Every read model that participates in the query layer must have an associated ID type.
- The framework should provide a trait (e.g., `QueryableReadModel` or similar) that ties a read model to its ID type and guarantees a default `get_by_id` query.

#### Query Declaration via Enum

- Each read model can define a query enum that enumerates all supported queries.
- Example:

```rust
enum OrderQuery {
    ById(OrderId),
    ByCustomer(CustomerId),
    PendingBefore(SystemTime),
    Search { status: Option<OrderStatus>, limit: usize },
}
```

- A trait (e.g., `Query`) would be implemented for this enum, associating it with the read model type and the response type for each variant.

#### Query Trait

Something along the lines of:

```rust
trait Query: Send {
    /// The read model this query targets.
    type ReadModel;
    /// The response type returned by executing this query.
    type Response: Send;
}
```

Or, if we want per-variant response types, the trait could use an associated enum for responses, or we could use a single response type that wraps variants.

#### QuerySpec (analogous to ConsumerSpec)

- A `QuerySpec` would declare how a set of queries for a read model are exposed.
- It could carry metadata like:
  - The read model type.
  - The query enum type.
  - Visibility (public/private).
  - Transport hints (e.g., expose via NATS request-reply, HTTP, gRPC, in-process only).
- Example:

```rust
let spec = QuerySpec::new::<OrderReadModel, OrderQuery>()
    .with_visibility(QueryVisibility::Public)
    .with_transport(QueryTransport::Nats);
```

#### Default By-ID Query

- The framework should ensure that any read model participating in the query layer automatically has a by-ID query, without the developer needing to manually add it to the enum.
- This could be done via a separate trait like `Identifiable` or `ReadModelId`:

```rust
trait ReadModelId {
    type Id: Send + Sync;
}
```

- The framework would provide a built-in query for `get_by_id` that does not need to appear in the user's query enum, or alternatively require it as a mandatory variant.

#### Query Handler

- A trait for handling queries, similar to how `Project` handles events:

```rust
trait QueryHandler: Send + Sync {
    type Query: Query;

    async fn handle(&self, query: Self::Query) -> Result<Self::Query::Response, Error>;
}
```

- The read model (or a dedicated query service) implements this trait.

### Open Questions

- Should the by-ID query be a built-in trait method on the read model, or a required variant in the query enum?
- ANSWER: built-in trait method, since every read model should define one.
- Should query responses be a single associated type, or should we support per-variant response types (which would likely need an enum or trait object)?
- ANSWER: queries will be operated on the READ MODEL, but not necessarily will return exactily the read model, a list or a filter could be applied, but if the implementation of this get complex, we should pivot to return only the read model or a array of read model potentially.
- How tightly should this integrate with the transport layer (NATS, Kurrent)? Should `QuerySpec` carry transport hints, or should transport mapping be entirely separate (like `ConsumerSpec` is transport-agnostic)?
- ANSWER: entirely separate, the spec can be defined inside the event modeling module of esrc
- Should queries support pagination, filtering, and sorting as first-class concepts, or leave that to the user's query enum variants?
- ANSWER: not really sure, maybe we can wrap the query in some helper types like PaginatedQueryResult<RM> or PaginatedQueryResult<SortedQueryResult<RM>>, but it's some a idea.
- How does caching fit in? Should the framework provide a caching layer around query handlers?
- ANSWER: not really sure, it could be done in so many ways, pick the best one.

### Follow-up Questions (Round 2)

Based on your answers above, here are additional questions to clarify the design before summarizing:

1. **Query Handler ownership**: Should the `QueryHandler` be implemented by the read model struct itself (e.g., `OrderReadModel` implements `QueryHandler`), or by a separate service struct that holds a reference to some storage backend (e.g., a `OrderQueryService` that wraps a database connection)? The distinction matters because read models built via `View` are in-memory and ephemeral, while projected read models via `Project` typically persist to external storage.
- ANSWER: not sure of the implications, i think if a read model is ephemeral only, it would probably never declare a project. and only a Query, but if you think of other usecases we might want to support you can decide the way yourself.

2. **In-memory vs. persistent read models**: For `View`-based read models (purely in-memory, built from replay), the query handler would need access to the materialized state. Should the framework provide a managed wrapper that holds the `View` state and serves queries against it? Or is that the user's responsibility?
- ANSWER: like the command_service feature, the framework should expose a interface to execute queries and get results, but the details are left for simple read models like "live projections" we can have a default implementation that the developer can plug in, would be nice for the nats to also expose a KV implementation since it has that resource, but the user should be able to plug their own storage and project data there freely.

3. **Query response type**: You mentioned queries operate on the read model but may return filtered/transformed results. Should we start simple with a single `type Response: Send` on the `Query` trait, and let the user define their response types freely? Or do you want the framework to enforce that responses are always the read model type (or `Vec<ReadModel>`)?
- ANSWER: you decide, the minimum requirement is Vec<ReadModel> since this works on all ours slices so far, but is not set in stone.

4. **Error type**: Should the `QueryHandler` use `esrc::error::Error`, or should it have its own associated `type Error` (like `Project` does), allowing domain-specific query errors (e.g., "not found", "invalid filter")?
- ANSWER: you decide

5. **ReadModelId trait and View**: You mentioned `View` could optionally implement `ReadModelId`. Currently `View` is generic over events but has no notion of identity or storage. Should `ReadModelId` be a separate trait entirely (not tied to `View`), so that both `View`-based and `Project`-based read models can opt in independently?
- ANSWER: you decide do the best for the framework structure in the future.

6. **get_by_id return type**: Should `get_by_id` return `Option<ReadModel>` (not found is `None`), or `Result<ReadModel, Error>` where not-found is an error variant?
- ANSWER: maybe both Result<Option<RM>> you decide that.

7. **Module placement**: You confirmed the spec should live in `event_modeling`. Should the `Query` trait, `QueryHandler` trait, and `ReadModelId` trait also live there, or should they have their own module (e.g., `src/query.rs`) since they represent a distinct concern from consumer declaration?
- ANSWER: separately in src/query.rs

### Follow-up Questions (Round 3)

Based on your round 2 answers, a few more clarifications before the final summary:

1. **Query transport exposure**: You mentioned for NATS we could expose a KV-backed implementation for simple read models. Should the KV store name be derived from the `ConsumerName` (similar to how durable consumer names are derived), or should the user provide it explicitly? This affects how "convention over configuration" the framework feels.
- ANSWER: you can decide on that.

2. **QueryHandler vs. QueryStore separation**: There seem to be two distinct concerns:
   - **QueryStore** (storage abstraction): knows how to `get_by_id`, `get_all`, `save`, etc. This is what the `Project` implementation writes to and what queries read from. Think of it as the persistence layer for a read model.
   - **QueryHandler** (query logic): takes a user-defined query enum and executes it, potentially using a `QueryStore` internally.
   
   Should the framework define both traits separately? This would let the framework provide built-in `QueryStore` implementations (in-memory, NATS KV, etc.) while the user only implements `QueryHandler` for custom queries. Or do you prefer a single unified trait?
- ANSWER: yes it should be two separate things, try to let the QueryStore be super flexible, since not all read models, need get_all for example.

3. **Live projection (View + replay) serving**: You mentioned that for simple "live projections" there could be a default implementation. Should this be a managed runtime component (similar to how `spawn_consumer` works) that:
   - Replays events to build the `View` state.
   - Subscribes to new events to keep it up to date.
   - Serves queries against the live in-memory state.
   
   Or should it be a simpler utility that the user manually wires together?
   - ANSWER: simpler utility with good ux

4. **Read model mutation from Project**: Currently `Project::project` receives events and can mutate external state. Should the `QueryStore` trait be what the `Project` writes into? For example, a projector would hold a `QueryStore` and call `store.save(id, read_model)` inside `project()`. This would unify the write path and read path around the same storage abstraction. Or should the write path remain entirely the user's responsibility inside their `Project` implementation?
   - ANSWER: i don't think we should merge these thinks, since we use project to run automations and other thinks that are not read models, only do this if you separate the project for exclusive use of read models, and create another trait for the other uses.



5. **Multiple query types per read model**: Can a single read model have multiple query enums (e.g., one for public API queries, another for internal/admin queries), or should it be strictly one query enum per read model?
- ANSWER: let's keep it simple for now, and only support a one enum


6. **get_by_id on the trait vs. on the query enum**: You confirmed `get_by_id` should be a built-in trait method. Should this method live on the `QueryStore` trait (storage-level concern, always available), on the `QueryHandler` (query-level concern), or on a `ReadModelId` trait that both can leverage? This determines where the "default query" lives architecturally.
- ANSWER: not sure but i think in the QueryStore is more intuitive?

### Relationship to Existing Types

- `View` could optionally implement `ReadModelId` to participate in the query layer.
  - This trait is not used for anything currently, if you need to refactor this in a more domain specific trait you could.
- `ReadModel` (the event_modeling builder) declares how events flow into the read model; `QuerySpec` would declare how data flows out.
- Together they form a complete vertical slice: events in via `ReadModel`/`ConsumerSpec`, data out via `QuerySpec`.
