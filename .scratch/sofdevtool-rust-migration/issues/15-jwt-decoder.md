# 15: Inspect JWT segments with History off by default

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Paste a JWT, inspect readable header and payload separately and optionally record it only after explicitly enabling History.

## Acceptance criteria

- [ ] Decode Base64URL and UTF-8 JSON with separate diagnostics for segment count, encoding and JSON failures; leave signature opaque.
- [ ] Prominently disclose no signature/claims/trust verification; provide no key input or verification action.
- [ ] Register JWT with History disabled by default in the shared recording path; invalid/intermediate payloads never persist.
- [ ] Enable explicit snapshot preview/restore only for retained entries; never automatically restore payloads on launch.
- [ ] Test fresh defaults, explicit opt-in, malformed segments and exact restoration without logging sensitive payloads.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. See [`rust/docs/evidence/ticket-15.md`](../../../rust/docs/evidence/ticket-15.md) for the gate results, native smoke checks and limits (macOS 14/15 runtime remain unverified).
