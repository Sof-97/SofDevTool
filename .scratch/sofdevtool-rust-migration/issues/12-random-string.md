# 12: Generate cryptographically random strings

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Configure an alphabet and batch, generate random strings, copy individual results or all results and restore exactly the generated values.

## Acceptance criteria

- [ ] Preserve default length 20/count one, upper/lowercase/digits/safe symbols and ambiguous-character exclusion, plus custom alphabets and saved controls.
- [ ] Use system cryptographic randomness with unbiased selection; validate alphabet, length and count before generation.
- [ ] Display entropy only when calculable and avoid password-management claims.
- [ ] Deliver complete Registry/session/Clipboard/snapshot behavior; record deliberate identical successful requests separately.
- [ ] Test alphabet/exclusion/bounds/Unicode invariants using deterministic randomness, not probabilistic quality assertions.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. See [`rust/docs/evidence/ticket-12.md`](../../../rust/docs/evidence/ticket-12.md) for the gate results, native smoke checks and limits (macOS 14/15 runtime remain unverified).
