# Ticket 02 implementation evidence

Status: implemented candidate for JSON, Base64 and Settings; awaiting coordinator integration and final independent review. Tickets 03/04 still own migration of the other Utility workspaces. No native launch was performed by this worker.

## Changed behavior

- `rust/crates/app/src/history.rs` now enforces a per-Utility recording pause at both `HistoryRecorder::record` and `HistoryStore::record`. Ordinary operations while paused return `HistoryError::Paused` before a new entry identity is created or a disk write begins. A failed atomic replacement keeps the previous file; temporary files are removed on failed writes. Mutations are serialized.
- `HistoryStore::retry` reads the exact retained set and writes it back without creating an operation. A failed retry keeps the pause; success clears it. Corrupt files stay untouched, and a new store instance resets only the in-memory pause. `clear_all_report` continues past an individual deletion failure and returns the actual cleared/failed Utility IDs, including unknown Utility files. Clear All uses paths obtained from the History root directory, so legacy filename IDs outside the current write/read ID rules are removable without accepting arbitrary paths.
- `HistoryRecorder` owns subscriptions and synchronous clear/retry notifications. `HistoryViewState` owns cached entries, selection, pending restore, warning state, retention invalidation and exact persisted-entry checks before restore. It reconciles the actual storage after successful or partial deletion while leaving each Utility's active controls, input and result alone. Status subscriptions redraw Settings after JSON/Base64 recording outcomes.
- `rust/crates/app/src/settings.rs` routes Clear Utility and Clear All through the coordinator, reports partial deletion honestly, and presents Retry only for paused Utilities. `rust/crates/app/src/json_workspace.rs` and `rust/crates/app/src/utilities/base64.rs` subscribe while open, including when hidden; they use the shared view state and reject stale selected/pending restores. Their History warnings distinguish corruption from a recording pause.
- The coordinator added the app `gpui` `test-support` dev dependency for GPUI entity regression tests. No other manifest change was made by this worker.

## Verification

All Cargo commands ran offline from `rust/` with `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target` and `CARGO_NET_OFFLINE=true`, using synthetic temporary History roots.

| Command | Result |
| --- | --- |
| `cargo test -p sofdevtool-app --lib` | 69 passed. Includes deterministic failed-write/failed-retry/successful-retry, no duplicate entry, preserved live session result, corrupt relaunch, policy independence, newest-25 retention, unknown and legacy filenames, partial deletion and shared selection invalidation. GPUI entity tests invoke the same clear/retry functions as Settings and verify notifications for visible and hidden subscribers. |
| `cargo check -p sofdevtool-app --all-targets` | Passed. |
| `cargo clippy -p sofdevtool-app --all-targets -- -D warnings` | Passed after replacing the explicit retry error propagation and adding safe legacy filename clearing. |
| `rustfmt --edition 2021 --check` on the four owned Rust files | Passed. |

The GPUI tests exercise real entity notifications with small subscriber probes, not rendered JSON/Base64 windows or native pointer/keyboard controls. Native Settings and inspector presentation, hold timing, VoiceOver and macOS 14/15 runtime behavior remain coordinator/final acceptance work.

## Migration API for tickets 03/04

For each remaining workspace, replace its four local History cache fields with `HistoryViewState::load(&history, Utility::ID)`. Keep the `HistorySubscription` returned by `history.subscribe(Utility::ID, callback)` for the workspace lifetime; its callback calls `view.reconcile(&history, Utility::ID)` and `cx.notify()`. Apply settled `record` results with `view.apply_record`, then call `history.notify_status(cx)` so an open Settings window refreshes its paused badge. Use `view.select` for list events and `view.retained(&history, Utility::ID, &entry)` immediately before any selected or pending restore. `HistoryRecorder::load`, `record` and `store` remain available for workspaces not yet migrated; the shared store already enforces pause for them.
