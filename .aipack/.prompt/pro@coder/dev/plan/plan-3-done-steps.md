## Step - Remove AggregateQueryHandler from esrc-cqrs
      status: done
time-created: 2026-03-20 20:49:02
   time-done: 2026-03-20 22:36:06

- Deleted `crates/esrc-cqrs/src/nats/aggregate_query_handler.rs`.
- Moved `QueryEnvelope` and `QueryReply` into `crates/esrc-cqrs/src/nats/query_dispatcher.rs` as public types; these are the canonical wire types for all query handlers.
- Removed `pub use aggregate_query_handler::{AggregateQueryHandler, QueryEnvelope, QueryReply};` from `mod.rs` and replaced with `pub use query_dispatcher::{NatsQueryDispatcher, QueryEnvelope, QueryReply};`.
- Removed `mod aggregate_query_handler;` and its associated comment from `mod.rs`.
- Removed the internal `use crate::nats::aggregate_query_handler::QueryReply;` from the error path in `query_dispatcher.rs` (now uses the locally defined `QueryReply`).
- Updated `crates/esrc-cqrs/tests/integration_nats.rs`: replaced all `AggregateQueryHandler` usages with a `counter_query_handler` helper function that implements `QueryHandler<NatsStore>` directly; updated imports.
- Updated `examples/cafe/main.rs`: replaced `AggregateQueryHandler` with an `order_state_query_handler` helper function; removed the unused `AggregateQueryHandler` and `ConnectOptions` imports; kept `OrderState::from_root` as the projection.

