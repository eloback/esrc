```yaml
#!meta (parametric prompt)

# See PATH_TO_PRO_CODER_DIR/README.md for additional documentation
# Note: All relative paths are relative to the workspace directory (parent folder of .aipack/)

## Static knowledge (relative & absolute)
knowledge_globs:
  # - /abs/or/rel/path/to/**/*.md      # Any relative or absolute path/glob for Markdown files
  # - pro@coder/README.md              # Pack path supported (here ask question about pro@coder)
  # - core@doc/for-llm/**/*.md         # For help building .aip AIPack agents
  # - pro@rust10x/guide/base/**/*.md   # Rust best practices (require `aip install pro@rust10x`)
  # - nats_service.md
  # - docs/skill/**/*

## Files the AI will work on (paths & content included in prompt, relative only)
context_globs:
  - Cargo.toml # for Rust
  - src/**/*
  # - examples/**/*
  - docs/skill/**/*
  # - examples/multi-slice-command-service/**/*
  # - examples/basic-query-service/**/*
  # - crates/esrc-cqrs/**/*
  # - derive/**/*.*
  # - compilation_errors.txt

#context_globs_post: # Appended after auto-context selection
# - .aipack/.prompt/pro@coder/dev/plan/*.md

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - src/**/*
  - examples/**/*
  # - crates/**/*

## Set to false to disable file writing (response below this file's prompt)
write_mode: true

## Optimize context files selection (other properties: code_map_model, helper_globs, ..)
auto_context:
  model: flash # (Use a small or inexpensive model)
  input_concurrency: 16 # (default 8)
  enabled: true # (Default true) Comment or set to true to enable.

dev:
  # chat: _workbench/queries # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  # plan: _workbench/integration_between_bounded_contexts # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: opus
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

Generate a skill file for http controller creation inside a slice.
here is how to setup a basic http controller inside a slice with our custom http server.

- the custom http_server crate is defined in the workspace.
- the custom error_responses_macro is defined in the workspace also.
- the http_server prelude export utoipa and other utils that composes all our http controllers

```rust
// http.rs
use super::*;
use crate::common::state::AppState;
use error_responses_macro::ErrorResponses;
use http_server::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct <FeatureName>Request {
    pub aggregate_id: Uuid,
    // other fields
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct <FeatureName>Response {
    pub aggregate_id: Uuid,
    // other fields
}

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

impl From<command::FeatureError> for ErrorResponses {
    fn from(_err: command::FeatureError) -> Self {
        ErrorResponses::InternalServerError
    }
}

const SPAN_NAME: &str = const_format::concatcp!(FEATURE_NAME, "::http");

#[instrument(name = SPAN_NAME, skip_all)]
#[utoipa::path(
    post,
    path = ENDPOINT,
    request_body = <FeatureName>Request,
    responses(
        (status = CREATED, body = <FeatureName>Response),
        ErrorResponses
    )
)]
pub async fn controller(
    State(state): State<AppState>,
    Json(body): Json<<FeatureName>Request>,
) -> Result<Json<<FeatureName>Response>, ErrorResponses> {
    let cmd = command::FeatureCommand {
        aggregate_id: body.aggregate_id,
        // ...
    };

    state.command_bus.dispatch(cmd).await?;

    Ok(Json(<FeatureName>Response {
        aggregate_id: body.aggregate_id,
        // ...
    }))
}
// setup.rs
use super::*;
use http_server::prelude::*;
use common::state::AppState;

pub fn setup(
    store: NatsStore,
    router: &mut OpenApiRouter<AppState>,
) {
    //...
    let new_router = std::mem::take(router).routes(routes!(http::controller));
    *router = new_router;
}
```

====
>   Info: Duration: 42s | ~$0.0982 | Prompt Tokens: 7,061 (cached: 0, cache_creation: 0) | Completion Tokens: 2,517 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 3 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git add -A . && git commit -m "+ docs/skill - Add esrc slice HTTP controller skill file"
</suggested_git_command>
