# 09: Generate text hashes with explicit execution

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Choose a digest and representation, explicitly hash the exact input bytes and restore the recorded digest.

## Acceptance criteria

- [ ] Support SHA-256/384/512, SHA-1 and MD5 with lower/upper hex and Base64 output; label SHA-1/MD5 as legacy.
- [ ] Keep untouched empty state neutral while explicit Hash of empty input processes zero bytes as one valid operation.
- [ ] Provide the complete Registry/session/Clipboard/History flow without HMAC, files or password-hashing scope.
- [ ] Test independent standard vectors, Unicode byte fidelity, representation choices, deliberate repeats and exact restore.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. See [`rust/docs/evidence/ticket-09.md`](../../../rust/docs/evidence/ticket-09.md) for the gate results, native smoke checks and limits (macOS 14/15 runtime remain unverified).
