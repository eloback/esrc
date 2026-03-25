====
>   Info: Duration: 16s | ~$0.0903 | Prompt Tokens: 14,896 (cached: 0, cache_creation: 0) | Completion Tokens: 631 
>  Model: claude-opus-4-6 | Adapter: Anthropic
>  Files: Context Files: 10 | Working Files: 0 | Knowledge Files: 2
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.



Looking at the current `reply_error` function in `src/nats/command_service.rs`, it uses `todo!()` and the design needs to change so that the NATS reply always sends `Ok(...)` (i.e., a successful NATS response), but the payload itself contains the error information. The caller then parses the response payload to determine success (empty body) or failure (JSON `CommandError`).

This also requires bounding `Serialize` on `CommandError` (already done) and ensuring the error is serialized into the response payload rather than using NATS service error mechanisms.

Let me create the plan.

<FILE_CHANGES>

<FILE_PATCH file_path="_workbench/service/plan-1-todo-steps.md">
@@
+## Step - Change reply_error to return Ok response with error payload instead of NATS service error
+      status: not_started
+time-created: 2026-03-25 18:27:32
+
+- Currently `reply_error` in `src/nats/command_service.rs` has a `todo!()` body and was intended to use the NATS service error response mechanism (status codes, error headers).
+
+- Change the approach so that the handler always replies with `request.respond(Ok(...))`:
+  - On success: respond with an empty payload (already done).
+  - On error: serialize the `CommandError` as JSON into the response payload and respond with `Ok(json_bytes)`.
+
+- The caller is then responsible for checking whether the response payload is empty (success) or contains a JSON `CommandError` (failure).
+
+- Update `reply_error` to serialize the `CommandError` via `serde_json::to_vec` and send it as the `Ok(...)` payload.
+
+- Ensure `CommandError` has `Serialize` bound (already has `#[derive(Serialize)]`), so no trait change needed there.
+
+- The `status_code()` method on `CommandError` can be kept for informational purposes or removed if no longer used by the NATS error mechanism. Keep it for now as it may be useful for callers.
+
+- The `serve` method return type remains `esrc::error::Result<()>`, meaning the method itself only returns `Err` for unrecoverable transport/setup failures. Per-request command errors are always sent back to the caller via the response payload.
</FILE_PATCH>

</FILE_CHANGES>

<suggested_git_command>
git commit -a -m ". service plan - Add todo step for reply_error payload-based error response"
</suggested_git_command>

