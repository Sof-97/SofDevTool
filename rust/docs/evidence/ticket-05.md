# Ticket 05 — History retention, deletion and storage failures

## Scope

Global and per-Utility recording preferences (disabling keeps existing entries),
deliberate deletion with a one-second pointer hold for Clear Utility and two
seconds for Clear All, and isolation of malformed files and paused Utilities.

## Implementation

- `HistoryPolicy { global_enabled, per_utility, defaults }` lives in the
  app-owned `HistoryRecorder`; the Registry supplies each Utility's default
  (`jwt-decoder` opts out). Disabling recording is a no-op that returns the
  retained entries unchanged, so nothing is deleted.
- The policy is persisted atomically in the fresh Rust namespace
  (`history-preferences.v1.json`) and loaded at startup; malformed data falls
  back to recording enabled.
- `HistoryStore` already isolates malformed files (never overwritten), pauses a
  Utility after a failed write, resumes on a successful retry or relaunch, and
  exposes `stored_utility_ids` so unknown Utility files can be listed and
  deleted only inside Rust History.
- The Settings window gained a History section: a global recording toggle, a
  per-Utility recording toggle and a one-second Clear hold, a two-second Clear
  All hold, unknown-file listing with Clear, and a confirmation banner for
  keyboard activation. The reusable `HoldButton` cancels on pointer release,
  pointer exit and never fires on keyboard activation (which shows the
  confirmation instead).

## Verification

- Unit tests cover policy gating, retention ordering, atomic-failure pause and
  resume, corruption isolation, unsafe-id rejection and the hold durations.
- Gate: `rust/scripts/verify --full` exit 0.
- Native hold/keyboard scenarios for the Settings History section are pending
  final host verification (see ticket 22 evidence).

## Limits

macOS 14/15 runtime unverified.
