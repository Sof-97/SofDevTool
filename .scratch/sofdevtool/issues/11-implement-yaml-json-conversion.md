Type: task
Status: resolved
Blocked by: 09

## What to build

Implement the YAML/JSON Conversion Utility as a complete local vertical slice. The owner can convert one YAML 1.2 document to JSON or JSON to YAML, explicitly swap direction, paste input, copy the canonical result, understand lossy round trips, and restore a completed valid operation from Utility History.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and the [implementation handoff](../implementation-handoff.md).

## Acceptance criteria

- [x] A source-defined Utility Definition with a stable ID, Format & Convert category, useful search aliases, and History enabled by default appears in the Library and Launcher without weakening the strongly typed module boundary.
- [x] Explicit YAML-to-JSON and JSON-to-YAML modes handle one document containing ordinary mappings, sequences, scalars, anchors, and aliases; multi-document streams receive a precise unsupported-input diagnostic.
- [x] JSON output is valid and deterministic. YAML values or mapping keys that cannot be represented faithfully in JSON are diagnosed instead of silently coerced.
- [x] The workspace visibly discloses that comments, aliases, formatting, and exact key presentation may not survive a round trip. It never claims lossless conversion.
- [x] Empty input is neutral. Malformed input replaces any stale result with a useful diagnostic; large input is never silently truncated or partially converted.
- [x] Paste, Copy Result, Clear, and direction switching are explicit and keyboard accessible. No clipboard read or write happens implicitly.
- [x] Each settled valid conversion records at most one versioned Utility Operation Snapshot. Preview and explicit restore reproduce controls, input, and result without rerunning or rerecording.
- [x] Yams 6.2.2 is exact-pinned and confined to the approved Utility-owned adapter; the app-owned YAML 1.2 Core resolution and JSON-fidelity layer satisfies the dependency assessment and resource-bound contract in the specification.
- [x] Contract-readable tests cover both directions, anchors/aliases, Unicode and complex emoji, scalar edge cases, malformed and multi-document input, lossy warnings, snapshot round-trip, stale-result rejection, registry/search behavior, and published or hand-reviewed fixtures.
- [x] `scripts/verify --full` passes and the ticket records the exact host/toolchain evidence without treating a macOS 14 deployment build as a runtime check.

## Answer

Implemented the YAML / JSON Utility as a complete strongly typed vertical slice with stable ID `yaml-json`, Format & Convert registration, explicit YAML → JSON and JSON → YAML directions, Swap/Paste/Copy/Clear actions, visible lossy-round-trip disclosure, revision-gated live conversion, and versioned History snapshot restore without recomputation or rerecording.

The app-owned conversion model accepts the JSON-compatible subset of YAML 1.2 Core, expands anchors and aliases, sorts object keys for deterministic JSON, preserves quoted/date-looking/YAML-1.1-looking strings, and diagnoses multi-document streams, malformed syntax, non-string or duplicate keys, non-finite/out-of-range numbers, unsupported tags, and excessive resources. The production policy is 1 MiB of UTF-8 input and 128 value-nesting levels; over-limit work publishes no result or History snapshot.

Yams is confined to `YamsYAMLDocumentCodec`. The Xcode project and `Package.resolved` exact-pin Yams 6.2.2 at revision `a27b21e0c81c5bf42049b897a62aaf387e80f279`. The checked-in notice generator and notice inventory include the Yams/bundled-libYAML MIT text. The Text Diff verifier now asserts PierreDiffsSwift's local immutable package reference specifically while separately asserting Yams's exact remote pin.

Verification on Apple Silicon macOS 26.2 (build 25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3:

- `xcodebuild -project SofDevTool.xcodeproj -list` passed and resolved Yams 6.2.2.
- Focused YAML/JSON suite: 8 tests passed.
- `scripts/verify --full` passed: formatting and dependency-policy checks, Debug and Release builds, 30 unit/integration tests, and 2 UI tests.
- Release `.app`: 16,276 → 17,536 KiB, a 1,260 KiB increase.
- Release executable: 2,887,760 → 4,181,392 bytes, a 1,293,632-byte increase.

The builds retain the macOS 14.0 deployment target, but this session performed runtime verification only on macOS 26.2. macOS 14 and 15 runtime verification remains pending on those hosts.
