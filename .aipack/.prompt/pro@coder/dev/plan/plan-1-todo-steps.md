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
