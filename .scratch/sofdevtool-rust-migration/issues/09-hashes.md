# 09: Generate text hashes with explicit execution

Type: task
Status: ready-for-agent
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

Pending implementation. The parent specification supplies shared behavior; this ticket makes no completion or runtime claim.
