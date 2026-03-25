# Dev Chat

Add a new `## Request: _user_ask_title_concise_` with the answer below (concise title). Use markdown sub-headings for sub sections. Keep this top instruction in this file. 

## Request: Replace CommandError with CommandReply and serializable Error

### Context

The `CommandService` error handling was implemented in `src/event/command_service.rs`, but there are issues to address:

- The file contains NATS-specific references (e.g., "NATS-compatible status code", "NATS reply payload", "NATS error status header", "NATS service endpoint"). These should be removed since `command_service.rs` lives in the base `esrc` crate and should be backend-agnostic.

- Currently, `CommandError` is defined in the base `esrc` crate. Instead, we should define a `CommandReply` that encapsulates both success metadata and an optional error.

### Proposed Interface

```rust
pub struct CommandReply {
    pub metadata: Value,
    pub error: Option<esrc::Error>,
}
```

### Requirements

- Remove all NATS-specific language from `src/event/command_service.rs`.
- Replace `CommandError` with a `CommandReply` struct that carries both metadata and an optional `esrc::Error`.
- Make `esrc::Error` (in `src/error.rs`) serializable (`Serialize` / `Deserialize`).

### Questions

- **`Value` type for metadata**: What type should `metadata` be? Options:
  - `serde_json::Value` (adds a hard dependency on `serde_json` in the base crate)
  - A generic `M: Serialize + DeserializeOwned`
  - A `HashMap<String, String>` or similar lightweight map
  - Something else?

- **Serializing `esrc::Error`**: The current `Error` enum wraps `BoxStdError` (i.e., `Box<dyn std::error::Error + ...>`) in its `Internal`, `External`, and `Format` variants. Trait objects are not serializable. To make `Error` serializable, we would need to either:
  - Convert the inner errors to strings (lossy, but simple), serializing only the `.to_string()` representation
  - Restructure the variants to hold serializable data instead of boxed trait objects
  - Which approach is preferred?

- **What about `CommandErrorKind`?**: Should `CommandErrorKind` be kept as a separate concept (perhaps renamed), folded into `esrc::Error` as new variants, or removed entirely in favor of just using `esrc::Error`?

- **`status_code()` method**: The current `status_code()` on `CommandError` maps to HTTP-style codes ("400", "409", etc.). Should this concept be moved to backend-specific code (e.g., NATS adapter), or should `esrc::Error` itself carry a status code mapping?

- **What does `CommandReply` carry on success?**: Currently, a successful command returns nothing. Should the metadata contain things like the new sequence number, the event name, or is it purely user-defined? Clarifying what goes into `metadata` on success would help shape the struct.
