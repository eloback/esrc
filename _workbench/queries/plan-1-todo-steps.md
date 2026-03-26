# Plan 1 - Todo Steps

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
