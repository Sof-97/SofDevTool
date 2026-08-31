Type: task
Status: resolved
Assignee: Codex
Blocked by: 09

## What to build

Implement Hashes as a text-first, local Utility. The owner can hash UTF-8 text with SHA-256, SHA-384, SHA-512, SHA-1, or MD5 and choose lowercase hexadecimal, uppercase hexadecimal, or Base64 output.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and the [implementation handoff](../implementation-handoff.md).

## Acceptance criteria

- [x] The stable Utility Definition is registered under Encode & Inspect with History enabled by default and appropriate hash/digest/checksum search aliases.
- [x] All five agreed algorithms produce a canonical digest for the exact UTF-8 bytes of the input in all three output formats.
- [x] SHA-1 and MD5 are visibly labelled legacy/non-security choices wherever selected; no algorithm is presented as password hashing or authentication.
- [x] Files, HMAC, password hashing, and additional checksums remain absent. The workspace neither reads files nor implies those capabilities.
- [x] Hash is an explicit action. Empty input remains neutral until the owner presses Hash, at which point the empty UTF-8 byte sequence is a deliberate valid operation.
- [x] Paste, Copy Result, Clear, algorithm, and output-format controls are explicit and keyboard accessible; deterministic updates reject stale work and record only settled valid operations.
- [x] Versioned History preview and explicit restore preserve algorithm, format, input, and digest without rerunning or rerecording.
- [x] Tests use published vectors for every algorithm, including empty bytes and Unicode UTF-8, verify hex casing and Base64, cover repeated operations and snapshots, and assert legacy labelling and registry discovery.
- [x] Prefer Apple system cryptography. Any added dependency satisfies and records the handoff's dependency review and exact-pin requirements.
- [x] `scripts/verify --full` passes and the implementation evidence names the verified host.

## Answer

The independent vertical slice is implemented under `SofDevTool/Utilities/Hashes` using Apple CryptoKit for SHA-256, SHA-384, SHA-512, SHA-1, and MD5. It provides an explicit Hash action, exact UTF-8 input, all three output formats, explicit Paste/Copy/Clear controls, visible legacy/non-security labelling, repeatable deliberate operations including empty input, and schema-version-1 snapshot restore without rerunning or rerecording.

The separate contract suite passed published `abc` and empty-input vectors for every algorithm and hand-reviewed complex-Unicode UTF-8 vectors for every algorithm, plus uppercase hexadecimal, Base64, repeated operations, legacy labels, and snapshot restore. The combined focused URL Encoding, Hashes, and Timestamps test command passed on Apple Silicon macOS 26.2 (25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3. Shared Registry/search and Utility-owned History preview integration are complete. The authoritative `scripts/verify --full` gate passed 84 unit/contract tests and 3 UI tests on that host.
