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

- ANSWER: plain serde is enough

2. **Aggregate loading** - To process a command, the current state of the aggregate must be replayed first. Should `aggregate_service` internally call `replay_one` to reconstruct the `Root<A>` before calling `process`, or does the caller supply an already-loaded root? If replay is internal, what is the starting sequence (always from 0, or passed in)?

- ANSWER: the service should call the read method from the ReplayExt trait and the try_write method from the PublishExt trait, the sequence is always 0.

3. **Aggregate ID routing** - Where does the aggregate ID come from? Options:
   - Embedded in the NATS request subject (e.g., `<prefix>.<aggregate_name>.<uuid>`).
   - Embedded in the request payload alongside the command.
   - Provided by the caller as a fixed ID (single-instance aggregate service).

- ANSWER: choose the best approach

- DECISION: Embed the aggregate ID in the NATS request subject, using the format `<aggregate_name>.<uuid>` as the endpoint subject. This mirrors how event subjects are already structured in the codebase (`NatsSubject`), keeps the command payload clean (only the command data), and allows NATS subject-based routing. The service endpoint will use a wildcard subject `<aggregate_name>.*` and extract the UUID from the last token of the incoming request subject.

4. **Reply payload** - What should the service reply with on success? Options:
   - The new `Sequence` after publishing the event.
   - The full resulting event (serialized).
   - An empty/ack-only response.
   - And on error, a structured error payload vs. a NATS error status?

- ANSWER: empty/ack-only response, and the error if possible a well structured error payload, it can be serialized manually by the user.

5. **OCC / concurrency** - Should the service use `publish` (with optimistic concurrency via `last_sequence`) or `publish_without_occ`? If OCC is used, does the service retry on sequence conflict, or return an error to the caller?

- ANSWER: should use try_write which calls publish with OCC, do not retry on conflict just error a conflict for the user to decide.

6. **Lifecycle / task model** - Should `aggregate_service` block (like `observe` does today) or spawn a background task and return a handle (similar to `durable_observe` + task tracker pattern already in `NatsStore`)? If a handle, should it integrate with the existing `GracefulShutdown` / `TaskTracker`?
   - ANSWER: implement both versions the handle can be exclusive for the NatsStore implementation

7. **Multi-command support** - A single aggregate may handle many command variants. The NATS Service API supports multiple named endpoints. Should each command variant map to its own endpoint, or should all commands for an aggregate share one endpoint and dispatch internally by a type header?
   - AWNSER: you can use only one endpoint, if it's more simple. if the simpler option is the separate endpoint, don't forget to add the aggregate as a prefix to the endpoint.

8. **Service naming** - Should the service name be the caller-supplied `SERVICE_NAME`, or should it be derived from the aggregate/event name (like subjects are derived from `E::name()`)? Same question for the endpoint name.
   - AWNSER: should be derived by the aggregate name if possible, if it will require a change to the Aggregate trait, you can ask the caller for a name.

### Design Summary

Based on all answers above, here is the consolidated design:

**Command serialization:** Plain `serde` (`Serialize` + `DeserializeOwned`) on `Aggregate::Command`. No new version trait needed.

**Aggregate loading:** The service internally calls `read` (from `ReplayOneExt`) to reconstruct `Root<A>` from sequence 0, then calls `try_write` (from `PublishExt`) to process the command and publish the event.

**Aggregate ID routing:** Embedded in the NATS request subject as `<aggregate_name>.<uuid>`. The endpoint listens on a wildcard subject and parses the UUID from the subject tail.

**Reply payload:** Empty body on success (ack-only). On error, a structured JSON payload is returned using a dedicated error response type that the caller can deserialize. NATS error status headers are also set.

**OCC / concurrency:** Uses `try_write` which calls `publish` with optimistic concurrency. No retry; a conflict is surfaced as an error to the caller.

**Lifecycle:** Two versions are provided. A blocking version (analogous to `observe`) and a spawned background task version that integrates with the existing `GracefulShutdown` / `TaskTracker` on `NatsStore`.

**Endpoint structure:** Single endpoint per aggregate, shared by all command variants. Dispatching is done internally by deserializing into `Aggregate::Command`. The endpoint subject uses the aggregate event name as prefix (from `A::Event::name()`), so no change to the `Aggregate` trait is needed for naming.

**Service name:** Derived from `A::Event::name()` at the NATS Service level. The caller does not need to supply a name.

### Implementation Plan

- Add a `CommandService` trait in a new `src/event/command_service.rs` (or `src/nats/command_service.rs`) with a blocking `serve` method.
- Add a `CommandServiceExt` with a `spawn_service` variant that integrates with `GracefulShutdown`.
- Implement both on `NatsStore`.
- Add a structured `CommandError` response type (serializable) for the error reply payload.
- The `Aggregate::Command` type must implement `serde::de::DeserializeOwned`; this bound is enforced at the call site, not on the trait itself.
