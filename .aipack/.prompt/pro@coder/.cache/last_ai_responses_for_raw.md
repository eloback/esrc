====
>   Info: Duration: 22s | ~$0.1202 | Prompt Tokens: 34,343 (cached: 0, cache_creation: 0) | Completion Tokens: 1,142 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 38 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at the current state of the plan files, I need to create a `plan-1-todo-steps.md` with the steps to build a CQRS example using `esrc-cqrs`. The active step is about fixing cargo check errors, so I'll plan the example steps to come after that work is done.

<FILE_CHANGES>

<FILE_PATCH file_path=".aipack/.prompt/pro@coder/dev/plan/plan-1-todo-steps.md">
@@
+## Step - Create the cafe example skeleton with domain types
+      status: not_started
+time-created: 2026-03-20 11:53:33
+
+Create the `examples/cafe/` directory structure with the core domain types for
+a cafe ordering system that will demonstrate `esrc-cqrs` usage.
+
+- Create `examples/cafe/main.rs` as the entry point (can be mostly empty stubs at this stage).
+- Create `examples/cafe/domain.rs` with:
+  - `Order` aggregate (fields: `status: OrderStatus`, etc.)
+  - `OrderCommand` enum (e.g., `PlaceOrder { item: String, quantity: u32 }`, `CompleteOrder`)
+  - `OrderEvent` enum deriving `Event`, `SerializeVersion`, `DeserializeVersion`, `Serialize`, `Deserialize`
+    (variants: `OrderPlaced { item: String, quantity: u32 }`, `OrderCompleted`)
+  - `impl Aggregate for Order`
+- Add the example entry in `Cargo.toml` under `[[example]]` pointing to `examples/cafe/main.rs`,
+  with `required-features = ["nats"]` (or similar).
+- Ensure the file compiles (`cargo check --example cafe`).
+
+## Step - Implement command handlers and projector for the cafe example
+      status: not_started
+time-created: 2026-03-20 11:53:33
+
+Add the CQRS wiring using `esrc-cqrs` types. References: domain types defined in
+plan-2-active-step.md (or plan-3-done-steps.md once done), step "Step - Create the cafe
+example skeleton with domain types".
+
+- Create `examples/cafe/projector.rs` with a `OrderProjector` struct implementing
+  `esrc::project::Project` for `OrderEvent`, printing or accumulating order activity.
+- Create `examples/cafe/handlers.rs` (or inline in `main.rs`) wiring up:
+  - `AggregateCommandHandler::<Order>::new("Order")` registered via `CqrsRegistry::register_command`.
+  - `DurableProjectorHandler::new("cafe-orders", OrderProjector::default())` registered via
+    `CqrsRegistry::register_projector`.
+- In `main.rs`:
+  - Connect to NATS, build `NatsStore`, build `NatsCommandDispatcher`, build `CqrsRegistry`.
+  - Spawn projectors via `registry.run_projectors()`.
+  - Start command dispatcher via `NatsCommandDispatcher::run(...)`.
+- Ensure the example compiles.
+
+## Step - Add a client driver to the cafe example
+      status: not_started
+time-created: 2026-03-20 11:53:33
+
+Add a small async block (or second binary / separate task spawned before the dispatcher) that
+acts as a client, sending commands over NATS request/reply so the example is fully self-contained
+and runnable with a local NATS server.
+
+References: wiring from plan-3-done-steps.md step "Step - Implement command handlers and
+projector for the cafe example".
+
+- In `main.rs`, before starting the dispatcher loop, spawn a Tokio task that:
+  - Sends a `PlaceOrder` command for a couple of items using `async_nats::Client::request`.
+  - Prints the `CommandReply` received.
+  - Sends a `CompleteOrder` command and prints the reply.
+  - Sleeps briefly so the projector has time to process the events.
+- Add a `README` section (or doc comment at the top of `main.rs`) explaining how to run
+  the example (`cargo run --example cafe`).
+- Ensure the full example compiles and the happy path works end-to-end.
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git add -A .
git commit -a -m "+ plan - Add cafe CQRS example plan steps"
</suggested_git_command>

