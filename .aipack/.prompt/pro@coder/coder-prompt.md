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
context_globs = ["src/**/*", "Cargo.toml", "derive/**/*"] 

# Relative to base_dir. Only include paths (not content) in the prompt.
# structure_globs = ["src/**/*.*"]

# Relative to base_dir. (optional) Files you actually want to work on, on top of the context files
# working_globs = ["**/*.js"]
# working_concurrency = true
# input_concurrency   = 6

# Note: This will add/override the model_aliases defined in .aipack/config.toml and ~/.aipack-base/config.toml
model_aliases = {gpro = "gemini-2.5-pro", flash = "gemini-2.5-flash", lite = "gemini-2.5-flash-lite-preview-06-17", claude = "claude-sonnet-4-20250514", gpt4 = "gpt-4.1", gpt="gpt-5"}

# Experimental flag to set the file content replace to search/replace when possible (can increase speed and decrease cost)
# xp_file_content_mode = "search_replace_auto" # default "whole"

# Set to true to write the files (otherwise, will show below the `====` )
write_mode = true

# It can be an alias name above, or model names like "o4-mini", "o4-mini-high".
# If not set, the model defined in config.toml will be used.  
model = "gpt"

# To see docs, type "Show Doc" and then press `r` in the aip terminal
```
make the DeserializeVersion trait be a owned and required to be implement for all types that implement Event or EventGroup.
the intent is to remove the lifetime in the TryFromEnvelope

====
>   Info: Duration: 5m 39s 487ms | Prompt Tokens: 24,296 | Completion Tokens: 16,820 (reasoning: 7,168) 
>  Model: gpt-5 (gpt-5-2025-08-07) | Adapter: OpenAI
>  Files: Context Files: 40 | Working Files: 0 | Knowledge Files: 0


