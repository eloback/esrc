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

## Files the AI will work on (paths & content included in prompt, relative only)
context_globs:
  # - package.json  # for Node.js
  - Cargo.toml # for Rust
  # - README.md
  - src/**/*
  - crates/esrc-cqrs/**/*
  # - derive/**/*.*
  # - compilation_errors.txt

context_globs_post: # Appended after auto-context selection
  - .aipack/.prompt/pro@coder/dev/plan/*.md

## File paths to give AI a broader view of the project (paths only in prompt, relative only)
structure_globs:
  - src/**/*
  - crates/**/*

## Set to false to disable file writing (response below this file's prompt)
write_mode: true

## Optimize context files selection (other properties: code_map_model, helper_globs, ..)
auto_context:
  model: gpt-5.4-mini # (Use a small or inexpensive model)
  input_concurrency: 8 # (default 8)
  enabled: true # (Default true) Comment or set to true to enable.

dev:
  chat: false # default path: PATH_TO_PRO_CODER_DIR/dev/chat/dev-chat.md
  plan: false # default  dir: PATH_TO_PRO_CODER_DIR/dev/plan/

## Full model names or aliases (see aliases ~/.aipack-base/config-default.toml)
## -high, -medium, or -low suffixes for custom reasoning (e.g., "flash-low", "opus-max", "gpt-high")
model: claude-sonnet-4-6
## (see PATH_TO_PRO_CODER_DIR/README.md for full pro@coder documentation)
```

Following the plan rules create a plan to: create a ServiceCommandHandler where it will define a service for a vertical slice or entire api to declare a service for it's own commands, don't forget to create a example and tests for this feature.
example for reference:

```rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Commands {
    RegistrarEscrituracao { id: Uuid, name: String, date: NaiveDate },
}
pub struct MyServiceHandler {
    external_api_handler: ExternalApi,
}
impl ServiceCommandHandler<Commands> for MyServiceHandler {
    ...
}
cqrs_registry.register_command(ServiceCommandHandler::<Commands>::new("my_api_service"))
```

====
>   Info: Duration: 49s | ~$0.1175 | Prompt Tokens: 22,183 (cached: 0, cache_creation: 0) | Completion Tokens: 3,396 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 9 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the plan rules and the existing plan files, I need to create/update `plan-1-todo-steps.md` with a plan for the `ServiceCommandHandler` feature, including examples and tests.

The existing plan already has two steps related to this feature. I need to review them and potentially refine or add to them based on the more detailed user instructions that include an example and tests requirement.

The user's example shows:
- A `ServiceCommandHandler<Commands>` trait (not generic over store in the impl side)
- `register_command(ServiceCommandHandler::<Commands>::new("my_api_service"))` - this suggests a wrapper struct
- The trait is implemented by the user's struct

Let me design a clean plan:

1. Define the `ServiceCommandHandler` trait and adapter struct
2. Add reply helper and client method
3. Add example and integration tests

The existing two steps cover points 1 and 2 but don't mention tests and examples explicitly. The user wants tests and examples added. I should revise the plan to incorporate these, potentially adding a third step for the example and tests, or folding them into the existing steps.

Looking at the existing steps, step 1 doesn't mention tests, and the example is mentioned only as a usage snippet. I'll update the plan to add a dedicated step for the cafe example extension and integration tests.



<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs plan - Add ServiceCommandHandler plan with trait, reply helper, example, and tests"
</suggested_git_command>
