Type: task
Status: resolved
Blocked by: 09

## What to build

Implement Regex as a responsive local Utility using the visibly identified platform ICU-compatible dialect. The owner can test a pattern against text, inspect all matches and capture groups, and preview replacement output without a pathological expression freezing the app.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and the [implementation handoff](../implementation-handoff.md).

## Acceptance criteria

- [x] The stable Utility Definition appears under Compare & Test with History enabled by default and regex/regular expression/ICU/match/replace aliases.
- [x] The workspace names the supported ICU-compatible dialect and exposes the agreed flags, pattern, test text, replacement template, all matches, numbered/named capture groups where available, and replacement preview.
- [x] Match ranges are Unicode-safe in user-visible text, unmatched optional groups are distinguishable from empty captures, and zero-length matches advance correctly without loops or duplicate phantom matches.
- [x] Invalid patterns and replacements produce precise diagnostics and remove stale results. Empty pattern/text states are explicitly defined and do not create accidental History entries.
- [x] A replaceable in-process ICU C backend enforces nonzero engine-step and heap-backtracking budgets, match/find callbacks, app-level size/result/output caps, cancellation/deadline, serial off-main execution, and superseded-revision rejection.
- [x] Production numeric budgets are calibrated with representative valid and pathological fixtures, recorded as named injectable policy values, and never described as an exact millisecond cutoff; resource failure publishes no partial result or History entry.
- [x] No cross-language flavor emulation or unsupported syntax translation is introduced.
- [x] Paste, Copy, Clear, Run/live behavior, flags, and replacement controls are explicit and keyboard accessible. Completed settled results emit at most one versioned snapshot.
- [x] History preview and explicit restore reproduce inputs, flags, matches/captures, and replacement preview without rerunning or rerecording.
- [x] Tests cover flags, multiple/overlapping expectations according to ICU semantics, captures, replacements, Unicode graphemes, zero-length matches, malformed patterns, resource bounds, cancellation/revision races, snapshots, and registry behavior using hand-reviewed fixtures.
- [x] `scripts/verify --full` passes; any interaction-heavy UI coverage uses predicate waits rather than timing sleeps, and evidence names the verified host.


## Answer

The Utility-owned vertical slice is implemented with a replaceable, in-process ICU C adapter and no `NSRegularExpression` fallback. `import ICU`, `URegularExpression`, callbacks, status mapping, and replacement calls are confined to `ICURegexBackend`. The app-owned domain exposes flags, matches, Unicode-safe UTF-16 ranges, empty-versus-unmatched captures, named captures, replacement preview, snapshots, cancellation, revision gating, and distinct resource diagnostics. One actor-owned worker serializes execution away from `MainActor`; superseded work receives an operation-local cancellation token.

Production policy calibration on Apple Silicon macOS 26.2 uses: 16,384 pattern UTF-16 units, 1,048,576 input units, 262,144 replacement units, 2,000,000 ICU engine steps, 8 MiB ICU heap backtracking, a 1.5-second monotonic callback guard, 10,000 matches, 50,000 captures, and 2,097,152 replacement-output units. These are named injectable work/resource limits, not a claim that ICU steps equal milliseconds or that callback latency is an absolute wall-clock cutoff. Focused fixtures passed for ordinary flags/captures/replacement/Unicode, zero-length progress, cancellation, one-step timeout, a 1 KiB deterministic heap overflow, cardinality/output caps, snapshots, and revision rejection.

Shared Registry and Utility-owned History preview integration are complete. Review fixes cancel superseded edits, hide stale output, distinguish lookbehind and inline options from named captures, and provide Copy confirmation. The authoritative `scripts/verify --full` gate passed 84 unit/contract tests and 3 UI tests on Apple Silicon macOS 26.2 (25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3. macOS 14 and 15 runtime verification remains pending.
