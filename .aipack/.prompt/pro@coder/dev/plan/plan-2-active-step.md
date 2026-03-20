## Step - Create the cafe example skeleton with domain types
      status: active
time-created: 2026-03-20 11:53:33
time-current: 2026-03-20 11:57:13

Create the `examples/cafe/` directory structure with the core domain types for
a cafe ordering system that will demonstrate `esrc-cqrs` usage.

- Create `examples/cafe/main.rs` as the entry point (can be mostly empty stubs at this stage).
- Create `examples/cafe/domain.rs` with:
  - `Order` aggregate (fields: `status: OrderStatus`, etc.)
  - `OrderCommand` enum (e.g., `PlaceOrder { item: String, quantity: u32 }`, `CompleteOrder`)
  - `OrderEvent` enum deriving `Event`, `SerializeVersion`, `DeserializeVersion`, `Serialize`, `Deserialize`
    (variants: `OrderPlaced { item: String, quantity: u32 }`, `OrderCompleted`)
  - `impl Aggregate for Order`
- Add the example entry in `Cargo.toml` under `[[example]]` pointing to `examples/cafe/main.rs`,
  with `required-features = ["nats"]` (or similar).
- Ensure the file compiles (`cargo check --example cafe`).
