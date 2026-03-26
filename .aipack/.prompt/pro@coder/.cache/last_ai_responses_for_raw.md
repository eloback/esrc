====
>   Info: Duration: 12s | ~$0.0805 | Prompt Tokens: 29,234 (cached: 0) | Completion Tokens: 492 
>  Model: gpt-5.4 (gpt-5.4-2026-03-05) | Adapter: OpenAIResp
>  Files: Context Files: 9 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git add -A . && git commit -m ". consumers - extend todo plan with multi-slice command service example step"
</suggested_git_command>

<FILE_CHANGES>
<FILE_PATCH file_path="_workbench/consumers/plan-1-todo-steps.md">
@@
 ## Step - expose ergonomic spawning helpers for automation and read model consumers
       status: not_started
 time-created: 2026-03-26 06:00:43
 
 Add high-level helpers that make startup code concise while keeping runtime ownership in infrastructure.
@@
 
 References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
+
+## Step - add a multi-slice example for command services and automation chaining
+      status: not_started
+time-created: 2026-03-26 06:07:23
+
+Create an example that declares at least two vertical slices and demonstrates the intended end-to-end workflow for the new consumer declaration model.
+
+- The example should:
+  - declare at least two slices
+  - start the `CommandService`
+  - execute at least one command manually
+  - have automations listen to the published event stream and trigger new commands
+  - keep running until ctrl-c is pressed
+
+- Keep the example aligned with the declaration layer and runtime layer split established by the earlier `event_modeling` steps, so the example validates the intended slice-facing ergonomics rather than exposing transport wiring directly.
+
+- Ensure the example is sequenced after the runtime and spawning helper work, since it depends on those pieces being available and should serve as a realistic usage reference.
+
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
 
 ## Step - document the event_modeling workflow and dev chat decisions
       status: not_started
 time-created: 2026-03-26 06:00:43
 
 Document how vertical slices should declare consumers with the new `event_modeling` module, and record the key decisions carried forward from the dev chat.
</FILE_PATCH>
</FILE_CHANGES>

