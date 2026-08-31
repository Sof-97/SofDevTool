Type: task
Status: resolved
Assignee: Codex
Blocked by: 09

## What to build

Implement URL Encoding as an independent local Utility for percent-encoding and decoding UTF-8. The owner chooses Encode or Decode and an explicit component or full-query-value mode, with no confusion between percent encoding and the existing URL-safe Base64 alphabet.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and the [implementation handoff](../implementation-handoff.md).

## Acceptance criteria

- [x] A stable source-defined Utility Definition appears under Encode & Inspect with aliases that find percent/URI/query encoding; History is enabled by default.
- [x] Encode/Decode and the approved Path Segment/Query Value modes are visible controls, with exact RFC-derived preserved/encoded character behavior rather than an opaque platform character set.
- [x] Encoding operates on UTF-8 and emits canonical uppercase percent triplets. Decoding rejects malformed triplets and invalid UTF-8 instead of guessing, replacing stale output with a precise diagnostic.
- [x] Tests pin the mode semantics for spaces, `+`, `%`, `&`, `=`, delimiters, non-ASCII text, combining marks, and complex emoji, and distinguish the result from URL-safe Base64.
- [x] Empty input is neutral; deterministic conversion runs live with appropriate debounce and only the latest input revision may publish output or History.
- [x] Paste, Copy Result, Clear, and optional Swap are explicit, keyboard accessible, and never access the clipboard implicitly.
- [x] Completed valid conversions emit one versioned snapshot; preview and explicit restore reproduce controls, input, and output without recomputation or a new History entry.
- [x] Registry, domain, malformed-input, Unicode, snapshot, and stale-result tests pass using independent standard vectors where available.
- [x] `scripts/verify --full` passes and the implementation evidence names the verified host.

## Answer

The independent vertical slice is implemented under `SofDevTool/Utilities/URLEncoding` with the strongly typed engine, debounced revision-gated session, explicit Paste/Copy/Clear/Swap controls, strict UTF-8 percent decoding, canonical RFC-derived encoding modes, and schema-version-1 snapshot restore that neither recomputes nor rerecords. Its separate contract suite pins path/query delimiters, literal percent and plus, `%20`, malformed triplets, invalid UTF-8, combining marks, non-Latin text, complex emoji, URL-safe-Base64 distinction, snapshot round trips, and stale revision rejection.

The combined focused URL Encoding, Hashes, and Timestamps test command passed on Apple Silicon macOS 26.2 (25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3. Shared Registry/search and Utility-owned History preview integration are complete. The authoritative `scripts/verify --full` gate passed 84 unit/contract tests and 3 UI tests on that host.
