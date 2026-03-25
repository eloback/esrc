## Step - Change reply_error to return Ok response with error payload instead of NATS service error
      status: active
time-created: 2026-03-25 18:27:32
time-current: 2026-03-25 18:31:14

- Currently `reply_error` in `src/nats/command_service.rs` has a `todo!()` body and was intended to use the NATS service error response mechanism (status codes, error headers).

- Change the approach so that the handler always replies with `request.respond(Ok(...))`:
  - On success: respond with an empty payload (already done).
  - On error: serialize the `CommandError` as JSON into the response payload and respond with `Ok(json_bytes)`.

- The caller is then responsible for checking whether the response payload is empty (success) or contains a JSON `CommandError` (failure).

- Update `reply_error` to serialize the `CommandError` via `serde_json::to_vec` and send it as the `Ok(...)` payload.

- Ensure `CommandError` has `Serialize` bound (already has `#[derive(Serialize)]`), so no trait change needed there.

- The `status_code()` method on `CommandError` can be kept for informational purposes or removed if no longer used by the NATS error mechanism. Keep it for now as it may be useful for callers.

- The `serve` method return type remains `esrc::error::Result<()>`, meaning the method itself only returns `Err` for unrecoverable transport/setup failures. Per-request command errors are always sent back to the caller via the response payload.

### Implementation

- Replaced `todo!()` in `reply_error` with `serde_json::to_vec(&error)` serialization, sending the JSON bytes as `request.respond(Ok(payload.into()))`.
- If serialization itself fails (unlikely), the error is logged and no reply is sent.
- If sending the reply fails, a warning is logged (matching the pattern already used for success replies in `serve`).
- Removed the outdated comment about NATS service error response not carrying a custom body, since we now send the payload as a normal Ok response.
