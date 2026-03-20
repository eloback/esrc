## Step - Implement command handlers and projector for the cafe example
      status: not_started
time-created: 2026-03-20 11:53:33

Add the CQRS wiring using `esrc-cqrs` types. References: domain types defined in
plan-2-active-step.md (or plan-3-done-steps.md once done), step "Step - Create the cafe
example skeleton with domain types".

- Create `examples/cafe/projector.rs` with a `OrderProjector` struct implementing
  `esrc::project::Project` for `OrderEvent`, printing or accumulating order activity.
- Create `examples/cafe/handlers.rs` (or inline in `main.rs`) wiring up:
  - `AggregateCommandHandler::<Order>::new("Order")` registered via `CqrsRegistry::register_command`.
  - `DurableProjectorHandler::new("cafe-orders", OrderProjector::default())` registered via
    `CqrsRegistry::register_projector`.
- In `main.rs`:
  - Connect to NATS, build `NatsStore`, build `NatsCommandDispatcher`, build `CqrsRegistry`.
  - Spawn projectors via `registry.run_projectors()`.
  - Start command dispatcher via `NatsCommandDispatcher::run(...)`.
- Ensure the example compiles.

## Step - Add a client driver to the cafe example
      status: not_started
time-created: 2026-03-20 11:53:33

Add a small async block (or second binary / separate task spawned before the dispatcher) that
acts as a client, sending commands over NATS request/reply so the example is fully self-contained
and runnable with a local NATS server.

References: wiring from plan-3-done-steps.md step "Step - Implement command handlers and
projector for the cafe example".

- In `main.rs`, before starting the dispatcher loop, spawn a Tokio task that:
  - Sends a `PlaceOrder` command for a couple of items using `async_nats::Client::request`.
  - Prints the `CommandReply` received.
  - Sends a `CompleteOrder` command and prints the reply.
  - Sleeps briefly so the projector has time to process the events.
- Add a `README` section (or doc comment at the top of `main.rs`) explaining how to run
  the example (`cargo run --example cafe`).
- Ensure the full example compiles and the happy path works end-to-end.
