====
>   Info: Duration: 15s | ~$0.0611 | Prompt Tokens: 24,512 (cached: 4,864) | Completion Tokens: 715 
>  Model: gpt-5.4 (gpt-5.4-2026-03-05) | Adapter: OpenAIResp
>  Files: Context Files: 9 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git add -A . && git commit -a -m ". consumers - plan dynproject compile fix investigation"
</suggested_git_command>

<FILE_CHANGES>
<FILE_PATCH file_path="_workbench/consumers/plan-1-todo-steps.md">
@@
+## Step - define the DynProject replacement direction and compile-safe abstraction
+      status: not_started
+time-created: 2026-03-26 07:00:52
+
+Document the intended replacement direction for the current `DynProject` design, focusing on a compile-safe abstraction that better matches the projector execution domain.
+
+- Evaluate whether the runtime should use boxed projector values directly, a new object-safe trait tailored to consumer execution, or another erased projector model that avoids the current generic/object-safety issues.
+
+- Define the responsibilities and constraints for the replacement abstraction, including:
+  - clone or factory behavior for per-message or per-task projector usage
+  - access to the event group names needed by consumer startup
+  - async message handling compatibility with the typed `Context` flow
+  - compatibility with sequential and concurrent NATS consumer execution
+
+- Capture the chosen direction in a way that subsequent implementation steps can reference without ambiguity.
+
+References: see the retained event modeling design context in `plan-3-done-steps.md`, step `Step - integrate event_modeling declarations with NatsStore consumer startup`.
+
+## Step - implement the new projector execution abstraction and remove DynProject compile errors
+      status: not_started
+time-created: 2026-03-26 07:00:52
+
+Replace the current `DynProject` implementation with the compile-safe abstraction defined in the previous step, updating the relevant runtime and declaration code so the crate builds correctly again.
+
+- Update the projector execution flow used by consumer declarations and `NatsStore` runtime helpers.
+
+- Ensure the implementation preserves the existing declaration-layer ergonomics where possible, while fixing object-safety, associated type, and async dispatch problems in the current design.
+
+- Remove or adapt the existing `DynProject` machinery only as needed to support the new execution model cleanly.
+
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.
+
+## Step - validate consumer declaration integration and document the updated projector model
+      status: not_started
+time-created: 2026-03-26 07:00:52
+
+Validate that the event modeling declarations still integrate correctly with the NATS runtime after the projector abstraction change, and document the retained design decisions.
+
+- Verify the updated execution model still supports:
+  - `ConsumerSpec<P>`
+  - `Automation<P>`
+  - `ReadModel<P>`
+  - sequential and concurrent consumer execution paths
+
+- Document the final projector execution approach, including why it replaced the previous `DynProject` design and how it better fits the consumer runtime domain.
+
+- Keep the documentation aligned with the declaration layer versus runtime layer split already established in the event modeling workflow.
+
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the DynProject replacement direction and compile-safe abstraction`.
</FILE_PATCH>
</FILE_CHANGES>

