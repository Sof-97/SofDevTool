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
- `rust/scripts/verify --full` exit 0 (see the integration commit).
- Native split/unified rendering, selection/copy and renderer failure/recovery
  were established for this renderer under ticket 02; the History panel was
  exercised natively in the final verification pass.

## Limits

macOS 14/15 runtime unverified. Cross-display scaling was not exercised.
