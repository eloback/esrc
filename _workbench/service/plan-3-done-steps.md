## Step - Add CommandError response type and command_service module skeleton

      status: done

time-created: 2026-03-25 15:39:07
time-done: 2026-03-25 17:54:46

- Created `src/event/command_service.rs` with the `CommandService` trait definition (blocking `serve` method).
- Defined a serializable `CommandError` struct and `CommandErrorKind` enum for structured error replies.
- Re-exported `CommandService`, `CommandError`, and `CommandErrorKind` from `src/event.rs` alongside the other event traits.

## Step - Implement blocking serve on NatsStore

      status: done

time-created: 2026-03-25 15:39:07
time-done: 2026-03-25 17:56:52

- Created `src/nats/command_service.rs` and implemented `CommandService::serve` for `NatsStore`.
- The endpoint subject is derived from `A::Event::name()`, using the wildcard pattern `<event_name>.*` to capture the aggregate UUID from the last subject token.
- On each request: parse the UUID from the subject, call `read` (`ReplayOneExt`) to load `Root<A>` from sequence 0, deserialize the command from the request payload, call `try_write` (`PublishExt`), reply with an empty body on success or a JSON-serialized `CommandError` with a NATS error status on failure.
- Added `status_code()` method to `CommandError` for mapping error kinds to HTTP-like status codes used by the NATS service error API.
- The `serve` method clones `NatsStore` for each request to obtain a mutable handle for `try_write`, since `NatsStore` is `Clone`.
- Service and group names are derived from `A::Event::name()`, endpoint is named `"command"`.

## Step - Add spawn_service background task variant on NatsStore

      status: done

time-created: 2026-03-25 15:39:07
time-current: 2026-03-25 18:11:52

- Defined `spawn_service` as an inherent method on `NatsStore` (not a trait, since it is NATS-specific).
- Uses `Tripwire` from `stream_cancel` for cancellation, matching the existing pattern in the codebase.
- The spawned task is registered with the `TaskTracker` and the `Trigger` is sent to the `exit_tx` channel so that `wait_graceful_shutdown` cancels it.
- `tokio::select!` is used to race between `serve` completing and the tripwire being triggered.
- References: see the definition in plan-3-done-steps.md, step "Step - Implement blocking serve on NatsStore".

## Step - Change reply_error to return Ok response with error payload instead of NATS service error
      status: done
time-created: 2026-03-25 18:27:32
   time-done: 2026-03-25 18:41:24

- Currently `reply_error` in `src/nats/command_service.rs` had a `todo!()` body and was intended to use the NATS service error response mechanism (status codes, error headers).

- Changed the approach so that the handler always replies with `request.respond(Ok(...))`:
  - On success: respond with an empty payload (already done).
  - On error: serialize the `CommandError` as JSON into the response payload and respond with `Ok(json_bytes)`.

- The caller is responsible for checking whether the response payload is empty (success) or contains a JSON `CommandError` (failure).

- Replaced `todo!()` in `reply_error` with `serde_json::to_vec(&error)` serialization, sending the JSON bytes as `request.respond(Ok(payload.into()))`.
- If serialization itself fails (unlikely), the error is logged and no reply is sent.
- If sending the reply fails, a warning is logged (matching the pattern already used for success replies in `serve`).
- Removed the outdated comment about NATS service error response not carrying a custom body, since we now send the payload as a normal Ok response.
- The `status_code()` method on `CommandError` is kept for informational purposes.

## Step - Update cafe example to use native esrc CommandService, remove esrc-cqrs dependency
      status: done
time-created: 2026-03-25 18:41:24
   time-done: 2026-03-25 18:41:24

- Rewrote `examples/cafe/main.rs` to use `NatsStore::spawn_service::<Order>()` for aggregate command handling instead of the `esrc-cqrs` command dispatcher.
- Replaced `CqrsClient` dispatch calls with direct NATS `request` calls to the command service endpoint subject.
- Replaced query dispatching with direct `store.read::<Order>(id)` replay calls.
- Removed `OrderProjector` and all projector/query registration logic.
- Deleted `examples/cafe/projector.rs`.
- Rewrote `examples/cafe/service.rs` to use a custom NATS service directly (no `esrc-cqrs` types), implementing `CafeServiceHandler::serve` as an async method that creates a NATS service and processes `CafeCommands`.
- Removed `OrderState` and `View` impl from `examples/cafe/domain.rs` since queries are now handled via direct aggregate replay.
- Updated `Cargo.toml` example required-features.
