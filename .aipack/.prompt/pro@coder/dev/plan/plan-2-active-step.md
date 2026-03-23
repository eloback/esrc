## Step - Define NatsServiceCommandHandler trait and adapter
      status: active
time-created: 2026-03-23 18:55:03
time-current: 2026-03-23 19:01:15

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
