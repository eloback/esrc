# Skill, Define an esrc vertical slice layout and constants

## Goal

Standardize how a vertical slice is structured in an esrc based project, ensuring:

- The slice is independent and does not import code from other slices.
- The slice declares `FEATURE_NAME` as a `const` at the root of the slice.
- The domain exposes `BOUNDED_CONTEXT_NAME` and `DOMAIN_NAME` constants.
- Instrumentation and consumer naming can reuse stable, consistent segments.

## Assumptions

- The bounded context and domain modules exist and are available as context to the implementer.
- A vertical slice is a feature folder, for example `dashboard_of_operations`, `send_email_to_user`, `create_operation`.
- You cannot import code from other vertical slices.
- The slice can import from the bounded context domain modules and from infrastructure (esrc, nats store wiring) as allowed by the project.

## Required constants

### Bounded context constant

File:

- `<bounded_context_name>/domain/mod.rs`

Required:

- `BOUNDED_CONTEXT_NAME` must exist and be a stable identifier for the bounded context.

Example:

- `const BOUNDED_CONTEXT_NAME: &str = <bounded_context_name>;`

### Domain constant

File:

- `<bounded_context_name>/domain/<domain_name>/mod.rs`

Required:

- `DOMAIN_NAME` must exist and be a stable identifier for the domain.

Example:

- `const DOMAIN_NAME: &str = <domain_name>;`

### Slice feature constant

File:

- `<bounded_context_name>/<domain_name>/<slice_folder>/mod.rs` (or the root module for the slice)

Required:

- Declare `FEATURE_NAME` in the root of the slice.

Example:

- `const FEATURE_NAME: &str = "criar_batch";`

Guidelines:

- The value should be a stable feature identifier, this is used for instrumentation and configuration.
- Prefer lowercase, and keep it stable once deployed, changing it changes consumer durable names and monitoring identifiers.

## Suggested slice folder layout

A minimal, scalable layout:

- `<slice_folder>/`
  - `mod.rs`
  - `generated.rs`
  - `read_model.rs` (optional)
  - `automation.rs` (optional)
  - `commands.rs` (optional helper types for slice owned commands, if any)
  - `wiring.rs` (optional, used to expose a single entry point to attach consumers to a store)

Notes:

- `generated.rs` is the public interface for read models and queries, and is typically overwritten by code generation.
- `mod.rs` re-exports public types, and is the best place to expose custom queries that are not generated.
- Keep the slice code cohesive, avoid splitting into many files early unless it is needed.

## Public API rules for the slice

- Treat `generated.rs` as part of the slice public API, it should be stable and ergonomic.
- Use `mod.rs` to re-export the generated types and also add custom queries and public entry points.

Example public exports pattern:

- `pub use generated::*;`
- `pub use read_model::...;`
- `pub use automation::...;`

## Anti patterns

- Importing a read model, automation, or internal helper type from another slice.
- Hiding `FEATURE_NAME` inside a submodule, it must be at the root to be consistently discoverable.
- Reusing a single consumer durable name across multiple features, it breaks operational isolation.
- Using unstable identifiers derived from environment variables as the constants, constants must be stable at compile time.
