# Plan 1 - Todo Steps

## Step - NATS KV-backed QueryHandler implementation
      status: not_started
time-created: 2026-03-26 15:39:52

- Create a concrete `QueryHandler` implementation backed by NATS JetStream Key-Value store.
- This provides a persistence layer for read models without requiring the developer to set up external storage.
- Design considerations:
  - The KV bucket name should be derived from the `ComponentName` (convention over configuration), with an option to override.
  - `get_by_id` reads from the KV bucket by key.
  - The `Project` implementation writes to the same KV bucket (the developer calls a save/put method inside their `project()` implementation).
  - Provide a helper or wrapper that the developer can hold inside their `Project` impl to write to the KV store, and that also implements `QueryHandler` for reads.
  - Serialization format: JSON (consistent with other NATS message payloads in the crate).
- Module placement: `src/nats/query_kv.rs` or similar, registered in `src/nats.rs`.
- References: see `src/query.rs` for `QueryHandler`, `src/nats.rs` for `NatsStore`.

## Step - Vertical slice helper composing ConsumerSpec and QuerySpec
      status: not_started
time-created: 2026-03-26 15:39:52

- Create a convenience builder or type that declares a `ReadModel` consumer (`ConsumerSpec`) and its `QueryHandler` (`QuerySpec`) together as a single vertical slice.
- This simplifies the developer experience for the common case where a read model has both an event consumer (write side) and a query handler (read side).
- Design considerations:
  - Should compose, not replace, the existing `ConsumerSpec` and `QuerySpec` types.
  - A builder pattern (e.g., `ReadModelSlice::new(name, projector, handler)`) that produces both specs, or a struct that holds both and can be passed to `NatsStore` for spawning.
  - Consider adding a `spawn_read_model_slice` or similar convenience method on `NatsStore` that spawns both the consumer and the query service in one call.
  - Be mindful of how these components are registered in the runtime (the user emphasized developer UX and runtime registration flow).
- Module placement: likely in `src/event_modeling.rs` since it is a declaration/composition concern, or a new `src/slice.rs` if it grows.
- References: see `src/event_modeling.rs` for `ConsumerSpec`, `ReadModel`, `Automation`; see `src/query.rs` for `QuerySpec`.

## Step - Caching decorator for QueryHandler
      status: not_started
time-created: 2026-03-26 15:39:52

- Create a `CachedQueryHandler<H: QueryHandler>` wrapper that adds TTL-based caching around any `QueryHandler` implementation.
- Design considerations:
  - Decorator pattern: wraps an inner `QueryHandler` and caches results from `get_by_id` (and optionally `handle`) with a configurable TTL.
  - Cache key for `get_by_id` is the serialized ID; cache key for `handle` would require the query to be hashable or serializable.
  - Start simple: cache only `get_by_id` results, leave `handle` uncached (or opt-in).
  - Use an in-memory cache (e.g., `HashMap` with expiry timestamps, or a lightweight LRU crate).
  - The wrapper should implement `QueryHandler` so it can be used as a drop-in replacement.
- Module placement: `src/query.rs` or a submodule like `src/query/cached.rs`.
- References: see `src/query.rs` for `QueryHandler`.

## Step - Pagination and sorting helper types for query responses
      status: not_started
time-created: 2026-03-26 15:39:52

- Create generic helper types for paginated and sorted query responses, such as `PaginatedResult<T>`, `SortedResult<T>`, or combined wrappers.
- Design considerations:
  - These are response wrapper types, not traits. They can be used as the `Response` associated type on `Query` implementations.
  - `PaginatedResult<T>` could contain: `items: Vec<T>`, `total: usize`, `offset: usize`, `limit: usize`, or cursor-based fields.
  - `SortedResult<T>` might just be a marker or carry sort metadata alongside items.
  - Keep it simple and optional; developers can use plain `Vec<ReadModel>` if they do not need pagination.
  - Should derive `Serialize`/`Deserialize` for transport compatibility.
- Module placement: `src/query.rs` or a submodule.
- References: see `src/query.rs` for `Query` trait's `type Response`.
