## Step - Define NatsServiceCommandHandler trait and adapter
      status: not_started
time-created: 2026-03-23 18:55:03

Add a `NatsServiceCommandHandler<C>` trait and a `ServiceCommandHandler<H, C>` adapter
struct to `esrc-cqrs` that allow a user to implement a single struct covering a whole
vertical slice or API group, dispatching a typed enum of commands under one NATS service
name, with one endpoint per enum variant (derived from the variant name).

- In `crates/esrc-cqrs/src/command.rs`:
  - Add trait `NatsServiceCommandHandler<S, C>` where `C: Serialize + DeserializeOwned + Send + Sync`:
    - Required method `fn name(&self) -> &'static str` returning the NATS service/group name.
    - Required async method `async fn handle(&self, store: &mut S, command: C) -> error::Result<Vec<u8>>`.
    - The trait is `Send + Sync + 'static` and generic over the store `S` like `CommandHandler<S>`.

- In `crates/esrc-cqrs/src/nats/command/service_command_handler.rs` (new file):
  - Define `ServiceCommandHandler<H, C>` adapter struct that:
    - Holds the user handler `H` (which implements `NatsServiceCommandHandler<S, C>`).
    - Holds the service name (`&'static str`) as the single NATS endpoint name, so all
      variants of `C` are received under the same subject.
    - Implements `CommandHandler<S>` by:
      1. Deserializing the raw payload as `C` via `serde_json`.
      2. Delegating to `H::handle(store, command)`.
      3. Returning the reply bytes verbatim (the user controls the reply shape entirely).
  - Provide `ServiceCommandHandler::new(handler: H) -> Self` constructor, using
    `handler.name()` as the endpoint name so there is a single registration call.

- In `crates/esrc-cqrs/src/nats/command/mod.rs`:
  - Add `pub mod service_command_handler;` and re-export `ServiceCommandHandler`.

- In `crates/esrc-cqrs/src/nats/mod.rs`:
  - Re-export `ServiceCommandHandler` from `nats::command`.

- In `crates/esrc-cqrs/src/lib.rs`:
  - Re-export `ServiceCommandHandler` at the crate root.

Intended usage pattern (the registration side):
```rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Commands {
    RegistrarEscrituracao { id: Uuid, name: String, date: NaiveDate },
}

pub struct MyServiceHandler { external_api: ExternalApi }

impl NatsServiceCommandHandler<NatsStore, Commands> for MyServiceHandler {
    fn name(&self) -> &'static str { "my_api_service" }
    async fn handle(&self, store: &mut NatsStore, command: Commands) -> error::Result<Vec<u8>> { ... }
}

registry.register_command(ServiceCommandHandler::new(MyServiceHandler { ... }));
```

## Step - Add ServiceCommandReply helper and CqrsClient dispatch method
      status: not_started
time-created: 2026-03-23 18:55:03

References: see the definition in plan-1-todo-steps.md,
step 'Step - Define NatsServiceCommandHandler trait and adapter'.

- In `crates/esrc-cqrs/src/nats/command/service_command_handler.rs`:
  - Add `ServiceCommandReply<R>` struct (generic over `R: Serialize + DeserializeOwned`) with
    fields `success: bool`, `data: Option<R>`, `error: Option<crate::Error>`.
  - Derive `Serialize`, `Deserialize`, `Debug`.
  - This is an opt-in convenience type; the user may use it or return any other serializable
    bytes from their `handle` implementation.
  - Add a `ServiceCommandReply::<()>::ok()` constructor and `ServiceCommandReply::err(e)` constructor.

- In `crates/esrc-cqrs/src/nats/client/cqrs_client.rs`:
  - Add method `dispatch_service_command<C, R>(&self, service_name, command_name, command: C) -> Result<R>`
    where `C: Serialize`, `R: DeserializeOwned`.
    - Sends the serialized command to subject `<service_name>.<command_name>`.
    - Deserializes the raw reply bytes as `R` and returns it.
  - This is intentionally low-level: the user chooses `R` to match whatever their handler returns.

- Update `crates/esrc-cqrs/src/lib.rs` re-exports to expose `ServiceCommandReply`.

## Step - Add cafe example extension and integration tests for ServiceCommandHandler
      status: not_started
time-created: 2026-03-23 18:55:03

References: see the definition in plan-1-todo-steps.md,
steps 'Step - Define NatsServiceCommandHandler trait and adapter' and
'Step - Add ServiceCommandReply helper and CqrsClient dispatch method'.

Add a concrete demonstration and automated tests for the new `ServiceCommandHandler`.

- Extend `crates/esrc-cqrs/examples/cafe/`:
  - Add a `service.rs` module containing:
    - A `CafeCommands` enum with at least two variants (e.g., `PlaceOrder { ... }`,
      `CompleteOrder { id: Uuid }`).
    - A `CafeServiceHandler` struct implementing `NatsServiceCommandHandler<NatsStore, CafeCommands>`.
    - The handler routes each variant to the appropriate domain logic using the store.
  - Wire `CafeServiceHandler` into `main.rs` via `registry.register_command(ServiceCommandHandler::new(...))`.
  - Add driver client calls using `CqrsClient::dispatch_service_command` to send each variant and
    print/assert the reply.

- In `crates/esrc-cqrs/tests/integration_nats.rs`:
  - Add a section `// -- ServiceCommandHandler tests --` with:
    - `test_service_command_handler_success`: send a well-formed command through the service
      handler, assert the reply has `success: true` and the store reflects the event.
    - `test_service_command_handler_error`: send a command that triggers a domain error,
      assert the reply has `success: false` and a populated `error` field.
    - `test_service_command_handler_malformed_payload`: send garbage bytes, assert that the
      dispatcher returns an error response and remains alive for subsequent requests.
  - Reuse the existing `TestCtx`, `spawn_dispatcher`, and `CounterEvent` types.
  - Implement a `CounterServiceHandler` that wraps `CounterCommand` dispatch through the
    service handler path, so the tests exercise the new code path end-to-end.

