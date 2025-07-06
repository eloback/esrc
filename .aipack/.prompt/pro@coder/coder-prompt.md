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
context_globs = ["package.json", "src/**/*.*"] 

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
write_mode = false

# It can be an alias name above, or model names like "o4-mini", "o4-mini-high".
# If not set, the model defined in config.toml will be used.  
model = "gpt"

# To see docs, type "Show Doc" and then press `r` in the aip terminal
```

====
> Ask your coding questions above the `====` delimiter, and press `r` in the terminal to replay.
>
> `coder` Agent parameters supported for this `coder` agent:
>
> `knowledge_globs`     - Allows you to add knowledge files to the context. These can be absolute or even refer to other packs,
>                         e.g., `["my-coding-guideline/**/*.md", "pro@rust10x/common/**/*.md"]`
>
> `base_dir`            - If activated in the TOML parameter section above, the context_globs and working_globs will be enabled.
>
> `context_globs`       - Customize your context globs relative to `base_dir` to decide which files are added to the context.
>                         If not defined, then no context files will be included in the prompt.
>                         These files will be described to the AI as `User's context files`.
>                         Narrowing the scope is better (both cost- and quality-wise, as it allows the model to focus on what matters).
>
> `working_globs`       - Customize your working globs to represent the working files. 
>                         When this is set, this allows the context_globs files to be explicitly cached (with Claude)
>                         and gives another opportunity to focus the AI on these files and treat the context_globs as just context.
>
> `structure_globs`     - Relative to base_dir. Only include paths (not content) in the prompt.
>                         This is very useful to give the AI the overall project shape, without using too much prompt context. 
>
> `working_concurrency` - When set to `true` and `working_globs` is defined, this will work on each working file concurrently,
>                         following the `input_concurrency` set in this section or in the workspace or aipack-base config.toml.
>
> `model_aliases`       - You can create your own alias names (which will override those defined in `.aipack/config.toml`).
>                         Top coder: "o3-mini-high" (aliased as 'high'), Fastest/~Cheapest: "gemini-2.0-flash-001".
>
> `model`               - Provide a direct model name or a model alias to specify which model this coder agent should use.
>
> Lines starting with `>` above the `====` or in the first lines just below the `====` will be ignored in the AI conversation.
> Here, give your instructions, questions, and more. By default, the code will be below.
>
> Feel free to remove these `> ` lines, as they are just for initial documentation and have no impact on AI instructions.
>
> You can ask, "Can you explain the coder agent parameters?" to get some documentation about them.
>
> Happy coding!
