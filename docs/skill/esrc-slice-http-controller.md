# Skill, Create an HTTP controller inside an esrc slice

## Goal

Standardize how an HTTP controller is created inside a vertical slice, ensuring:

- The controller lives inside the slice folder as `http.rs`.
- The slice setup wires the controller route into the shared `OpenApiRouter`.
- Request and response types are defined locally in the slice, with OpenAPI schema derivation.
- Error responses follow a consistent pattern using the `error_responses_macro` crate.
- Tracing instrumentation uses stable slice constants.
- The controller delegates to command dispatch or query logic; it does not contain business rules.

## Assumptions

- The workspace provides the `http_server` crate, which re-exports `utoipa`, `axum` extractors (`State`, `Json`), `OpenApiRouter`, `routes!`, `ToSchema`, and other utilities through its prelude.
- The workspace provides the `error_responses_macro` crate, which exposes the `ErrorResponses` derive macro.
- The slice already declares `FEATURE_NAME` as a `const` at its root module (see `esrc-slice-constants-and-module-layout` skill).
- The slice defines an `ENDPOINT` constant that holds the HTTP path for this feature.
- The application state type (`AppState`) is defined in a shared `common::state` module and is accessible from the slice.

## Slice file layout with HTTP controller

Extending the standard slice layout:

- `<slice_folder>/`
  - `mod.rs`
  - `http.rs`
  - `setup.rs`
  - `generated.rs`
  - `read_model.rs` (optional)
  - `automation.rs` (optional)

Notes:

- `http.rs` contains the controller function, request/response structs, and error response enum.
- `setup.rs` contains the route wiring function that attaches the controller to the router.
- Both files are part of the slice and must not import from other slices.

## HTTP controller file structure

### File: `<slice_folder>/http.rs`

#### Imports

Required imports:

- `use super::*;` to access slice-level constants and sibling modules.
- `use crate::common::state::AppState;` for the shared application state.
- `use error_responses_macro::ErrorResponses;` for the error response derive.
- `use http_server::prelude::*;` for axum extractors, utoipa macros, and router utilities.
- `use serde::{Deserialize, Serialize};` for request and response serialization.
- `use tracing::instrument;` for instrumentation.
- `use uuid::Uuid;` if aggregate identifiers are used.

#### Request type

Define a request struct:

- Derive `Debug`, `Serialize`, `Deserialize`, `ToSchema`.
- Name it `<FeatureName>Request`.
- Include only the fields the HTTP caller must provide.

Example:

```rust
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateOperationRequest {
    pub aggregate_id: Uuid,
    // other fields specific to the feature
}
```

#### Response type

Define a response struct:

- Derive `Debug`, `Serialize`, `Deserialize`, `ToSchema`.
- Name it `<FeatureName>Response`.
- Include only the fields the HTTP caller needs to receive.

Example:

```rust
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateOperationResponse {
    pub aggregate_id: Uuid,
    // other fields specific to the feature
}
```

#### Error responses enum

Define an error enum:

- Derive `Debug`, `Clone`, `Serialize`, `utoipa::ToSchema`, `ErrorResponses`.
- Annotate with `#[error_responses(endpoint = ENDPOINT)]`.
- Each variant uses `#[error(status = <code>, title = "<title>")]`.
- Common variants: `ValidationError`, `Conflict`, `InternalServerError`.

Example:

```rust
#[derive(Debug, Clone, Serialize, utoipa::ToSchema, ErrorResponses)]
#[error_responses(endpoint = ENDPOINT)]
pub enum ErrorResponses {
    #[error(status = 400, title = "Validation Error")]
    ValidationError { detail: String },

    #[error(status = 409, title = "Conflict")]
    Conflict,

    #[error(status = 500, title = "Internal Server Error")]
    InternalServerError,
}
```

Implement `From` conversions from domain errors to `ErrorResponses`:

```rust
impl From<esrc::Error> for ErrorResponses {
    fn from(_err: esrc::Error) -> Self {
        match _err {
            esrc::Error::External(err) => ErrorResponses::ValidationError { detail: err.to_string() },
            ...
        }
    }
}
```

Guidelines for error mapping:

- Map domain validation errors to `400`.
- Map concurrency conflicts to `409`.
- Map unexpected or infrastructure errors to `500`.
- Do not expose internal error details in the response body unless they are safe for the caller.

#### Span name constant

Define a span name using `const_format::concatcp!`:

```rust
const SPAN_NAME: &str = const_format::concatcp!(FEATURE_NAME, "::http");
```

This ensures tracing spans are consistently named and discoverable.

#### Controller function

Define an `async fn controller` that:

- Is annotated with `#[instrument(name = SPAN_NAME, skip_all)]` for tracing.
- Is annotated with `#[utoipa::path(...)]` for OpenAPI documentation.
- Accepts `State(state): State<AppState>` and `Json(body): Json<Request>`.
- Returns `Result<Json<Response>, ErrorResponses>`.
- Constructs and dispatches a command, or executes a query.
- Maps the result into the response type.

Example:

```rust
const SPAN_NAME: &str = const_format::concatcp!(FEATURE_NAME, "::http");

#[instrument(name = SPAN_NAME, skip_all)]
#[utoipa::path(
    post,
    path = ENDPOINT,
    request_body = CreateOperationRequest,
    responses(
        (status = CREATED, body = CreateOperationResponse),
        ErrorResponses
    )
)]
pub async fn controller(
    State(state): State<AppState>,
    Json(body): Json<CreateOperationRequest>,
) -> Result<Json<CreateOperationResponse>, ErrorResponses> {

    /// command or query dispatch normally

    Ok(Json(CreateOperationResponse {
        aggregate_id: body.aggregate_id,
    }))
}
```

Guidelines:

- The controller must not contain business logic; it translates HTTP to commands/queries and back.
- Use the `?` operator with `From` conversions to keep error handling concise.
- Prefer `CREATED` (201) for commands that create resources, `OK` (200) for queries or updates.

## Setup file structure

### File: `<slice_folder>/setup.rs`

The setup function wires the controller route into the application router.

Required imports:

- `use super::*;` to access the `http` module.
- `use http_server::prelude::*;` for `OpenApiRouter` and `routes!`.
- `use common::state::AppState;` for the state type.

Function signature:

```rust
pub fn setup(
    store: NatsStore,
    router: &mut OpenApiRouter<AppState>,
) {
    // any store or consumer wiring for the feature

    let new_router = std::mem::take(router).routes(routes!(http::controller));
    *router = new_router;
}
```

Guidelines:

- The `setup` function is the single entry point for attaching this slice to the application.
- Use `std::mem::take` and reassign to avoid ownership issues with mutable router references.
- If the slice also has consumers or automations, wire them in the same `setup` function.
- The `store` parameter (or equivalent infrastructure handle) is passed in so the slice can set up any event consumers or projections it needs.

## Module re-exports

In `<slice_folder>/mod.rs`, declare the submodules and re-export the setup function:

```rust
const FEATURE_NAME: &str = "create_operation";
const ENDPOINT: &str = "/operations";

mod http;
pub mod setup;
```

Guidelines:

- `FEATURE_NAME` and `ENDPOINT` are at the root of the slice so all submodules can access them via `super::*`.
- `http` is private; the controller is exposed only through the `setup` function.
- `setup` is public so the application wiring can call it.

## Anti-patterns

- Putting business logic in the controller function; keep it in the aggregate or a domain service.
- Defining request/response types outside the slice; they belong to the slice that owns the endpoint.
- Using string literals for span names instead of deriving them from `FEATURE_NAME`.
- Sharing `ErrorResponses` enums across slices; each slice defines its own.
- Importing `http.rs` or `ErrorResponses` from another slice.
- Forgetting to wire the route in `setup.rs`, which leaves the endpoint unreachable.
- Using `unwrap` or `expect` in the controller; always return typed errors through `ErrorResponses`.

## Testing approach

- Unit test request-to-command mapping independently from the HTTP layer.
- Integration test the endpoint by:
  - Building the router with `setup`.
  - Sending an HTTP request using `axum::test` or an equivalent test client.
  - Asserting the response status code and body.
  - Asserting error responses for validation failures, conflicts, and internal errors.
- Verify OpenAPI schema generation includes the endpoint, request body, and all response variants.
