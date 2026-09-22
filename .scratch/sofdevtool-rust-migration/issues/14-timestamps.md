# 14: Convert timestamps without timezone guesses

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Interpret Unix/ISO/local time explicitly, inspect the same instant in selected zones and restore the captured instant.

## Acceptance criteria

- [ ] Implement Auto/manual modes and exact digit-count inference from the parent spec; require explicit seconds mode for fractional seconds.
- [ ] Require offset/Z for ISO Auto and a named zone for local time; reject repeated/nonexistent DST wall times.
- [ ] Expose Now through an injectable clock, label inferred input and timezone, and retain locale-stable output.
- [ ] Deliver full Registry/session/Clipboard/History behavior; restore never reads the clock again.
- [ ] Test negative/fractional/boundary values, DST gaps/folds, known timezone fixtures and exact snapshots.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. See [`rust/docs/evidence/ticket-14.md`](../../../rust/docs/evidence/ticket-14.md) for the gate results, smoke checks and limits (macOS 14/15 runtime remain unverified).
