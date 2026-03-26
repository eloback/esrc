# Plan 2 - Active Step

## Step - In-memory QueryHandler helper for View-based live projections
      status: active
time-created: 2026-03-26 15:39:52
time-current: 2026-03-26 16:23:08

- Created `src/query/in_memory.rs` with `InMemoryViewStore<RM, Q>`, a thread-safe, in-memory store for read model instances keyed by `Uuid`.

- `InMemoryViewStore` provides:
  - `new(query_fn)`: constructor that takes a closure for custom query logic. The closure receives the query enum and an immutable `&HashMap<Uuid, RM>` snapshot.
  - `upsert(id, model)`: insert or update a read model instance.
  - `remove(id)`: remove a read model instance.
  - `get(id)`: get a cloned read model by ID.
  - `all()`: get all entries as a `Vec`.
  - `len()` / `is_empty()`: size introspection.

- Implements `QueryHandler` for `InMemoryViewStore<RM, Q>`:
  - `type Query = Q`, `type Id = Uuid`
  - `get_by_id` delegates to `self.get(&id)`.
  - `handle` acquires a read lock and calls the user-supplied `query_fn` closure.

- Converted `src/query.rs` to `src/query/mod.rs` (module directory) to accommodate the new `in_memory` submodule. All existing content from `src/query.rs` is preserved verbatim in `src/query/mod.rs`.

- Design decisions:
  - The store is cheaply cloneable (`Arc`-wrapped internals), so it can be shared between a `Project` impl (write side) and a `QuerySpec` (read side).
  - No coupling to `View` trait directly; the store is a generic read model container. The user materializes their `View` (or any read model) and calls `upsert` in their `Project::project` method.
  - Custom query logic is injected via a closure rather than requiring the user to implement a separate trait, keeping the UX simple.
  - The `query_fn` closure pattern avoids the need for the user to create a separate handler struct for simple cases, while still allowing full flexibility.
