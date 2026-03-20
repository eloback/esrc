====
>   Info: Duration: 13s | ~$0.0248 | Prompt Tokens: 4,925 (cached: 0, cache_creation: 0) | Completion Tokens: 667 
>  Model: claude-sonnet-4-6 | Adapter: Anthropic
>  Files: Context Files: 3 | Working Files: 0 | Knowledge Files: 0
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

Looking at `crates/esrc-cqrs/src/nats/mod.rs`, I can see two issues:

1. The module-level doc comments for `aggregate_command_handler`, `aggregate_projector_handler`, and `aggregate_query_handler` are missing (only `command_dispatcher`, `projector_runner`, and `query_dispatcher` have `pub mod` with docs).
2. The private `mod` declarations for `aggregate_command_handler`, `aggregate_projector_handler`, and `aggregate_query_handler` are not documented, and more importantly, `query_dispatcher` is declared `pub mod` but the others that are re-exported via `pub use` are only private `mod` declarations, which is inconsistent and may cause visibility issues depending on usage. The re-exports should be sufficient, but the doc comments on the submodules are missing entirely.

The main fix needed: add doc comments for `aggregate_command_handler`, `aggregate_projector_handler`, and `aggregate_query_handler` modules, and ensure `query_dispatcher` is consistently documented.

<FILE_CHANGES>

<FILE_PATCH file_path="crates/esrc-cqrs/src/nats/mod.rs">
@@
 /// NATS-backed CQRS dispatcher and projector runner.
@@
 /// NATS command dispatcher backed by core NATS request/reply service groups.
 pub mod command_dispatcher;
 /// NATS projector runner backed by JetStream durable consumers.
 pub mod projector_runner;
+/// NATS query dispatcher backed by core NATS request/reply service groups.
+pub mod query_dispatcher;

 pub use aggregate_command_handler::{AggregateCommandHandler, CommandEnvelope, CommandReply};
 pub use aggregate_projector_handler::DurableProjectorHandler;
 pub use aggregate_query_handler::{AggregateQueryHandler, QueryEnvelope, QueryReply};
 pub use command_dispatcher::NatsCommandDispatcher;
 pub use query_dispatcher::NatsQueryDispatcher;
 pub use projector_runner::NatsProjectorRunner;

+/// Aggregate command handler wiring: maps a typed command to an aggregate and writes events.
 mod aggregate_command_handler;
+/// Aggregate projector handler wiring: maps a projector to a durable JetStream consumer.
 mod aggregate_projector_handler;
+/// Aggregate query handler wiring: maps a typed query to an aggregate replay and returns a response.
 mod aggregate_query_handler;
-pub mod query_dispatcher;
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git commit -a -m "^ esrc-cqrs/nats - Add missing module doc comments and fix query_dispatcher visibility"
</suggested_git_command>

