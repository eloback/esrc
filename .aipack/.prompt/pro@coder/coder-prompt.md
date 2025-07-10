```toml
#!meta - parametric agent block

# Paths or globs relative to the workspace directory (the directory containing `.aipack/`),
# or absolute paths, including those starting with `~/` for home directories.
# These will be referenced as "knowledge files".
# knowledge_globs = ["path/to/knowledge/**/*.md", "core@doc/**/*.md", "pro@rust10x/guide/base/**/*.md"]

# If not set, context_globs and working_globs won't be evaluated
base_dir = "" # Leave empty for workspace root; make sure to narrow context_globs

# Relative to base_dir. Inline these files’ contents into the prompt (narrow as the project grows)
# (e.g., for Rust, replace "package.json" with "Cargo.toml")
context_globs = ["src/**/*", "Cargo.toml"] 

# Relative to base_dir. Only include paths (not content) in the prompt.
# structure_globs = ["src/**/*.*"]

# Relative to base_dir. (optional) Files you actually want to work on, on top of the context files
# working_globs = ["**/*.js"]
# working_concurrency = true
# input_concurrency   = 6

# Note: This will add/override the model_aliases defined in .aipack/config.toml and ~/.aipack-base/config.toml
model_aliases = {gpro = "gemini-2.5-pro", flash = "gemini-2.5-flash", lite = "gemini-2.5-flash-lite-preview-06-17", claude = "claude-sonnet-4-20250514", gpt = "gpt-4.1"}

# Experimental flag to set the file content replace to search/replace when possible (can increase speed and decrease cost)
# xp_file_content_mode = "search_replace_auto" # default "whole"

# Set to true to write the files (otherwise, will show below the `====` )
write_mode = true

# It can be an alias name above, or model names like "o4-mini", "o4-mini-high".
# If not set, the model defined in config.toml will be used.  
model = "claude"

# To see docs, type "Show Doc" and then press `r` in the aip terminal
```

Add observability to the crate, adding some instrument macros from the tracing crate.
The default level will be debug
Only add attribute if it's free as we most interest in capture the performance of the crate.
Add instrument(skip_all) to all relevant nats implementations.
Add instrument(skip_all) to all TraitExt (as those are not in nats)

====
>   Info: Duration: 2m 2s 390ms | ~$0.1444 | Prompt Tokens: 15,946 (cached: 0, cache_creation: 2,395) | Completion Tokens: 6,318 
>  Model: claude-sonnet-4-20250514 | Adapter: Anthropic
>  Files: Context Files: 19 | Working Files: 0 | Knowledge Files: 0

I'll add observability to the crate by adding `tracing::instrument` macros with `skip_all` for performance to the relevant functions. Here are the files that need to be updated:







The changes I've made:

1. **Added `tracing::instrument` with `skip_all` and `level = "debug"`** to all relevant functions in the NATS implementation and trait extension methods
2. **Added `use tracing::instrument;`** imports to the modified files
3. **Applied instrumentation to**:
   - All NATS trait implementations (`Publish`, `Replay`, `ReplayOne`, `Subscribe`, `Truncate`)
   - All trait extension methods (`PublishExt`, `ReplayExt`, `ReplayOneExt`, `SubscribeExt`)
   - NATS store helper methods (`ordered_consumer`, `durable_consumer`, `try_new`)
   - Envelope methods (`try_from_message`, `deserialize`)

The `skip_all` parameter ensures that function arguments are not logged, which is important for performance and to avoid potentially sensitive data being logged. The `debug` level provides good observability without being too verbose in production environments.

