```toml
#!meta - parametric agent block

# Paths or globs relative to the workspace directory (the directory containing `.aipack/`),
# or absolute paths, including those starting with `~/` for home directories.
# These will be referenced as "knowledge files".
knowledge_globs = ["nats_service.md"]

# If not set, context_globs and working_globs won't be evaluated
base_dir = "" # Leave empty for workspace root; make sure to narrow context_globs

# Relative to base_dir. Inline these files’ contents into the prompt (narrow as the project grows)
# (e.g., for Rust, replace "package.json" with "Cargo.toml")
context_globs = ["src/**/*", "Cargo.toml", "derive/**/*", "crates/esrc-cqrs/**/*"]

# Relative to base_dir. Only include paths (not content) in the prompt.
# structure_globs = ["src/**/*.*"]

# Relative to base_dir. (optional) Files you actually want to work on, on top of the context files
# working_globs = ["**/*.js"]
# working_concurrency = true
# input_concurrency   = 6

# Note: This will add/override the model_aliases defined in .aipack/config.toml and ~/.aipack-base/config.toml
# model_aliases = {gpro = "gemini-2.5-pro", flash = "gemini-2.5-flash", lite = "gemini-2.5-flash-lite-preview-06-17", claude = "claude-sonnet-4-5-20251001", gpt4 = "gpt-4.1", gpt="gpt-5"}

# Experimental flag to set the file content replace to search/replace when possible (can increase speed and decrease cost)
# xp_file_content_mode = "search_replace_auto" # default "whole"

# Set to true to write the files (otherwise, will show below the `====` )
write_mode = true

# It can be an alias name above, or model names like "o4-mini", "o4-mini-high".
# If not set, the model defined in config.toml will be used.
model = "claude-sonnet-4-6"

# To see docs, type "Show Doc" and then press `r` in the aip terminal
```

create a esrc extension crate named "esrc-cqrs" that will allow the registry of command and event handlers using the Command(command_handler) and Project(event_handler) from the base esrc crate.
create a nats implementation of the extension, reason about the best construct from nats to use for each one.

====
>   Info: Duration: 1m 39s | ~$0.2075 | Prompt Tokens: 32,920 (cached: 0, cache_creation: 0) | Completion Tokens: 7,250 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 40 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

I'll design and implement the `esrc-cqrs` extension crate. Let me reason through the design first.

## Design Reasoning

**Command Handler (CQRS Command Side):**
- Commands are imperative, targeted at a specific aggregate instance
- They need request/reply semantics since the caller wants confirmation
- NATS **Request/Reply** (via JetStream services or core NATS) is the natural fit
- Subject pattern: `<prefix>.cmd.<AggregateName>.<CommandName>`
- The handler loads the aggregate, processes the command, publishes the event, and replies

**Event Handler (Project side):**
- Event handlers are projectors that react to published events
- They need durable, at-least-once delivery for reliability
- NATS **JetStream durable consumers** (push or pull) are the right choice, same as the existing `Subscribe` implementation in `esrc`
- The registry manages multiple projectors attached to different event groups

**Registry Design:**
- A `CqrsRegistry` holds both command handlers and event handler projectors
- It uses the existing `esrc` traits: `Aggregate`, `Project`, `Publish`, `ReplayOne`
- Command handler: loads aggregate via `ReplayOneExt::read`, processes via `PublishExt::try_write`
- Event handler: delegates to existing `SubscribeExt::observe` or `ReplayExt::rebuild`

**Workspace structure:**
- New crate at `crates/esrc-cqrs/`
- NATS impl lives within it behind a feature flag



Here is a summary of what was created:

**`crates/esrc-cqrs/`** - new extension crate with:

- `src/command.rs` - `CommandHandler<S>` trait: takes raw bytes, returns raw bytes, named for routing
- `src/projector.rs` - `ProjectorHandler<S>` trait: named durable handler, runs indefinitely
- `src/registry.rs` - `CqrsRegistry<S>`: stores handlers behind object-safe erased wrappers, drives projectors via `JoinSet`, exposes `run_projectors()`
- `src/nats/command_dispatcher.rs` - uses **NATS Services API** (request/reply service groups), one endpoint per handler; correct choice because commands are synchronous, targeted, and do not need durability
- `src/nats/projector_runner.rs` - thin wrapper delegating to `durable_observe`; correct choice because projectors need durable at-least-once delivery with resumable position
- `src/nats/aggregate_command_handler.rs` - `AggregateCommandHandler<A>`: loads aggregate via `ReplayOneExt::read`, writes via `PublishExt::try_write`, wraps command in `CommandEnvelope { id, command }`
- `src/nats/aggregate_projector_handler.rs` - `DurableProjectorHandler<P>`: wraps any `Project` impl with a durable name, delegates to `durable_observe`

<suggested_git_command>
git add -A .
git commit -a -m "+ esrc-cqrs - Add CQRS extension crate with command and event handler registry with NATS implementation"
</suggested_git_command>
