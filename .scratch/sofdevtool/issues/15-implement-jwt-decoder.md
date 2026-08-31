Type: task
Status: resolved
Blocked by: 09

## What to build

Implement JWT Decoder as a deliberately non-validating, local inspection Utility. It decodes and presents readable header and payload segments without accepting a key or secret and without making any statement about signature, claims, trust, validity, or fitness for use.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and [Design Utility History storage and privacy](06-design-history-storage-and-privacy.md).

## Acceptance criteria

- [x] The stable Utility Definition appears under Encode & Inspect with JWT/token/header/payload aliases and declares History supported but disabled by default.
- [x] The workspace decodes the Base64URL header and payload as UTF-8 JSON, presents them separately in a readable deterministic form, and treats the signature segment as opaque and unverified.
- [x] A persistent, prominent explanation says that decoding does not verify a signature, interpret claims, judge validity, or establish trust. There is no secret/key input and no verification action.
- [x] Malformed segment counts, invalid Base64URL, invalid UTF-8, and invalid header/payload JSON produce segment-specific diagnostics and remove stale decoded content.
- [x] Empty input is neutral. Paste, separate Copy actions, and Clear are explicit and keyboard accessible; the clipboard is never read or written implicitly.
- [x] Input and decoded payloads are neither persisted nor restored by default. If the owner explicitly enables JWT History, completed valid decodes may create versioned snapshots and are restored only through an explicit History action, never automatically on launch.
- [x] Snapshot preview/restore respects the privacy controls, restores without rerecording, and never logs, indexes, exports, or otherwise exposes JWT payloads.
- [x] Tests cover compact tokens with Unicode JSON, omitted padding, malformed segments, algorithm-confusion-shaped fixtures without interpreting them, disabled-default History, explicit opt-in, restoration, stale-result rejection, and registry discovery.
- [x] `scripts/verify --full` passes and the implementation evidence names the verified host.


## Answer

The Utility-owned vertical slice is implemented: strict three-segment Base64URL decoding, separate UTF-8/JSON diagnostics for header and payload, deterministic readable JSON, an opaque signature, a prominent non-validation warning, explicit clipboard actions, revision-gated live work, and versioned restore that applies the captured result without rerunning or rerecording. Focused JWT, Case Conversion, and Regex tests pass on Apple Silicon macOS 26.2 with Xcode 26.6 / Swift 6.3.3.

The Registry and AppModel recording path now preserve JWT's disabled-by-default History setting until the owner explicitly opts in, and the Utility-owned decoder renders History previews. The authoritative `scripts/verify --full` gate passed 84 unit/contract tests and 3 UI tests on Apple Silicon macOS 26.2 (25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3. macOS 14 and 15 runtime verification remains pending.
