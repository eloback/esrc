# Dev Chat

Add a new `## Request: _user_ask_title_concise_` with the answer below (concise title). Use markdown sub-headings for sub sections. Keep this top instruction in this file. 

## Request: CommandService trait for aggregate command handling via NATS Service API

### Requirement Summary

Introduce a `CommandService` trait (analogous to `Subscribe`/`SubscribeExt` for projectors) that wires an `Aggregate` to a NATS Service endpoint. The `NatsStore` will expose a method `aggregate_service` that returns (or internally creates) an `async_nats::Service`, sets up an endpoint for the given aggregate type, receives commands over that endpoint, loads or constructs the aggregate root, invokes `process` + `publish`, and replies with the result.

**Proposed call site:**

```rust
let _ = store.aggregate_service::<MyAggregate>(SERVICE_NAME, SERVICE_VERSION).await;
```

This mirrors the `observe`/`project` pattern:
- `observe` = subscribe to a stream of events, run a projector on each.
- `aggregate_service` = listen on a NATS service endpoint, deserialize a command, run the aggregate, publish the resulting event, reply with success/error.

### Open Questions

1. **Command transport format** - Commands are not currently modeled as a trait in this codebase (only `Aggregate::Command` exists as an associated type with no serialization bound). How should commands be serialized over NATS? Should a `SerializeCommand` / `DeserializeCommand` version trait be introduced (mirroring `SerializeVersion` / `DeserializeVersion` for events), or is plain `serde` (`Serialize` + `DeserializeOwned`) sufficient?

2. **Aggregate loading** - To process a command, the current state of the aggregate must be replayed first. Should `aggregate_service` internally call `replay_one` to reconstruct the `Root<A>` before calling `process`, or does the caller supply an already-loaded root? If replay is internal, what is the starting sequence (always from 0, or passed in)?

3. **Aggregate ID routing** - Where does the aggregate ID come from? Options:
   - Embedded in the NATS request subject (e.g., `<prefix>.<aggregate_name>.<uuid>`).
   - Embedded in the request payload alongside the command.
   - Provided by the caller as a fixed ID (single-instance aggregate service).

4. **Reply payload** - What should the service reply with on success? Options:
   - The new `Sequence` after publishing the event.
   - The full resulting event (serialized).
   - An empty/ack-only response.
   - And on error, a structured error payload vs. a NATS error status?

5. **OCC / concurrency** - Should the service use `publish` (with optimistic concurrency via `last_sequence`) or `publish_without_occ`? If OCC is used, does the service retry on sequence conflict, or return an error to the caller?

6. **Lifecycle / task model** - Should `aggregate_service` block (like `observe` does today) or spawn a background task and return a handle (similar to `durable_observe` + task tracker pattern already in `NatsStore`)? If a handle, should it integrate with the existing `GracefulShutdown` / `TaskTracker`?

7. **Multi-command support** - A single aggregate may handle many command variants. The NATS Service API supports multiple named endpoints. Should each command variant map to its own endpoint, or should all commands for an aggregate share one endpoint and dispatch internally by a type header?

8. **Service naming** - Should the service name be the caller-supplied `SERVICE_NAME`, or should it be derived from the aggregate/event name (like subjects are derived from `E::name()`)? Same question for the endpoint name.
