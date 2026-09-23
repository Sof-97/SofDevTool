# Ticket 21 — Text Diff sessions and History

## Scope

The proven local web renderer is integrated with real Registry metadata, an
explicit Clipboard surface and versioned snapshots. A completed current
comparison is recorded once in fresh Rust History; restore reproduces the exact
old/new texts and mode without rerunning the renderer readiness handshake.

## Implementation

- `sofdevtool_core::utilities::text_diff` owns `TextDiffRequest { old, new,
  mode }`, `TextDiffEvaluation` and `TextDiffSnapshot { old, new, mode }`, with
  `impl Utility` (`ID = "text-diff"`, `SNAPSHOT_VERSION = 1`). Both sides empty
  is neutral; one-sided comparisons are valid.
- The workspace records only when the renderer reports Ready for the current
  revision and the revision has not already been recorded. The initial sample
  comparison is not a user operation and is not recorded.
- Restore sets both editors and the mode directly, suppresses the change-driven
  re-render, re-renders once, and marks the revision as recorded so restore
  never records.
- Loading/ready/error states, stale-callback rejection via the monotonic
  revision, the disclosed complex-emoji whole-line fallback, Copy original /
  Copy updated, and the History panel with confirmed restore are all present.

## Verification

- Core tests cover neutral/valid evaluation and exact snapshot round-trip
  including complex emoji.
- Native (macOS 26.2 arm64): pasting an updated side recorded one Text Diff
  entry (`2/25` after a second edit); selecting an older entry and restoring it
  reproduced the exact old/new texts and mode, re-rendered the diff, and did not
  add a History entry (`2/25` stayed `2/25`).
- Gate: `rust/scripts/verify --full` exit 0.

## Limits

macOS 14/15 runtime unverified. Cross-display scaling was not exercised.
