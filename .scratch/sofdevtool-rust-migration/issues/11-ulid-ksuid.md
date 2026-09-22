# 11: Add ULID and KSUID to Identifier Generator

Type: task
Status: resolved
Blocked by: 10

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Select ULID or KSUID within Identifier Generator, generate or inspect values and restore complete results with the selected format.

## Acceptance criteria

- [ ] Expose only format-relevant controls; ULID/KSUID are formats, not UUID versions.
- [ ] Support ULID random/process-local monotonic generation, canonical uppercase, strict validation and timestamp inspection.
- [ ] Support cryptographically random KSUID generation, case-sensitive validation, timestamp inspection and explicit ordered-sequence batches capped at 65,536.
- [ ] Retain UUID workflows, per-item/Copy All and exact History restoration across all formats without fresh randomness or time.
- [ ] Test fixed-time boundaries, monotonic behavior, encoding/overflow, batch limits and snapshot round-trips with deterministic fixtures.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. See [`rust/docs/evidence/ticket-11.md`](../../../rust/docs/evidence/ticket-11.md) for the gate results, smoke checks and limits (macOS 14/15 runtime remain unverified).
