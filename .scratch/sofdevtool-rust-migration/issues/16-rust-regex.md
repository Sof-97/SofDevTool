# 16: Test patterns and replacements with Rust regex

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Test a pattern against text, inspect matches/captures and replacement preview with clearly identified Rust regex semantics.

## Acceptance criteria

- [ ] Use the pinned regex crate and document actual supported flags/replacement syntax; reject unsupported look-around/backreferences clearly with no ICU emulation.
- [ ] Apply named limits to pattern compilation, input, match/capture counts and replacement output; calibrate and record representative boundaries.
- [ ] Run bounded work off the UI thread, coalesce obsolete requests and revision-gate all results/snapshots; check cancellation where controllable without promising hard engine interruption.
- [ ] Handle UTF-8 offsets, absent captures and zero-length matches correctly; resource failures keep input and publish neither partial success nor History.
- [ ] Deliver full Registry/session/Clipboard/snapshot flow with supported/unsupported syntax, Unicode, replacement and stale-completion tests.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. See [`rust/docs/evidence/ticket-16.md`](../../../rust/docs/evidence/ticket-16.md) for the gate results, native smoke checks and limits (macOS 14/15 runtime remain unverified).
