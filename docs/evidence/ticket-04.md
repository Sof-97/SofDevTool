# Ticket 04 — fresh Rust History for JSON

## Scope

One versioned JSON History file per Utility under the fresh Rust Application
Support namespace (`SofDevToolRust/History/<utility>.history.v1.json`). Writes
use a same-directory temporary file and an atomic rename; a failed write keeps
the last valid file and pauses that Utility. Malformed files are isolated and
never overwritten. The shared `Session<U>` rejects stale revisions and exposes
one snapshot per settled valid operation, so preview and restore never rerun.

## Storage contract

- `HistoryEntry { id, captured_at, utility_id, snapshot_version, payload }` with
  a Utility-owned opaque payload. `payload` is never logged, indexed, synced or
  exported.
- `HistoryStore` (app-owned) validates the Utility id as a single safe filename
  component, rejects a file that declares another Utility, retains the newest 25
  entries ordered by timestamp then descending id, and lists unknown Utility
  files for deliberate deletion.
- `HistoryRecorder` stamps entries from an injectable `HistoryClock`; tests use a
  fixed clock, production uses system time plus an in-process sequence.
- Restore is confirmed only when replacing a different non-empty session, then
  applies the captured request/output directly and records nothing.

## Reproducible native checks

Host: macOS 26.2 (25C56), Apple Silicon arm64; Rust 1.98.1. Debug bundle,
isolated `SOFDEVTOOL_RUST_SUPPORT_ROOT`, synthetic Clipboard/Accessibility input.

1. Paste `{"b":2,"a":[1,2,3],"c":{"nested":true}}`; the formatted result appears
   and History shows `1/25` with the captured timestamp and an output preview.
   `json.history.v1.json` contains the versioned envelope with the exact
   Utility-owned snapshot.
   See `docs/evidence/previous-migration/assets/04-json-record.png`.
2. Paste `{"x":1}`, then `{"y":9}`; each settled valid operation records once.
3. Relaunch the bundle; all three entries reload exactly (`3/25`).
4. Select the oldest entry and `Restore selected`; the confirmation banner
   appears because the current session differs. `Restore` reproduces the exact
   input and output and does not add a History entry (`3/25` stays `3/25`).
5. Native testing exposed and fixed a real crash: the confirmation button
   reused the History restore focus handle, so GPUI aborted with
   `set_focus called more than once in a single frame`. The confirmation now
   owns a distinct focus handle.

## Verification boundaries

- macOS 14 and 15 runtime remain unverified.
- Unknown-snapshot listing, retention tie order, atomic-failure pause,
  corruption isolation and unsafe-id rejection are covered by `HistoryStore`
  unit tests rather than native scenarios.

## Automated verification

`CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target rust/scripts/verify --full`
exit 0: rustfmt, Clippy `-D warnings`, 66 tests (30 core, 8 app, 23 JSON
contract, 5 UI), Debug and Release builds. `Session<U>` tests cover one-shot
snapshots, stale rejection and restore-without-reevaluation; `HistoryStore`
tests cover round-trip, retention, corruption, pause/resume and clear.
