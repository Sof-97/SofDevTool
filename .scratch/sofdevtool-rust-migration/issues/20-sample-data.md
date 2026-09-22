# 20: Generate fictional structured sample data

Type: task
Status: resolved
Blocked by: 10

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Edit typed fields, generate fictional JSON or CSV rows and restore the exact schema and generated output.

## Acceptance criteria

- [ ] Support name/email/number/boolean/date/UUID/enum fields with meaningful bounded options, reordering and duplicate/invalid-name validation.
- [ ] Default to ten JSON rows; enforce 1–1,000 rows and at most 50 fields with stable field order.
- [ ] Generate valid JSON and documented CSV quoting for delimiters, quotes, newlines, Unicode and empty values; label names/emails fictional.
- [ ] Use core identifier generation and injected randomness/clock seams; no cross-UI dependency, external datasets, relations or schema import.
- [ ] Provide full Utility integration, reusable applicable field-editor primitives, deterministic fixtures and exact restore without regeneration.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. See [`rust/docs/evidence/ticket-20.md`](../../../rust/docs/evidence/ticket-20.md) for the gate results, smoke checks and limits (macOS 14/15 runtime remain unverified).
