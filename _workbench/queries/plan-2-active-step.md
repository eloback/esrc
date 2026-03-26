# Plan 2 - Active Step

## Step - NATS KV-backed QueryHandler implementation
      status: active
time-created: 2026-03-26 15:39:52
time-current: 2026-03-26 16:28:37

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

