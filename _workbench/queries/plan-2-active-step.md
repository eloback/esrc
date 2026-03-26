# Plan 2 - Active Step

## Step - Rename ConsumerName to ComponentName in event_modeling.rs
      status: active
time-created: 2026-03-26 14:58:37
time-current: 2026-03-26 15:04:08

- Renamed the `ConsumerName` struct to `ComponentName` in `src/event_modeling.rs`.
- Renamed the `consumer` field to `component` (and updated the associated `consumer()` method to `component()`).
- Updated `durable_name()` and `slice_path()` to use the new field name.
- Updated all references in `ConsumerSpec`, `Automation`, `ReadModel` builders within the same file.
- Kept `ConsumerRole`, `ConsumerSpec`, `Automation`, and `ReadModel` names unchanged.
- Updated all usages across the codebase: example files that reference `ConsumerName` or the `consumer` field/method.
- `src/nats.rs` and `src/nats/command_service.rs` do not directly reference `ConsumerName` by import (they use `ConsumerSpec` which internally holds `ComponentName`), so no changes needed there.
