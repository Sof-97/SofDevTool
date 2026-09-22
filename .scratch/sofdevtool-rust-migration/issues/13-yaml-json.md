# 13: Convert YAML and JSON with fidelity diagnostics

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Convert one YAML or JSON document, understand unsupported or lossy cases and restore the exact conversion.

## Acceptance criteria

- [ ] Select and pin a maintained parser after checking the app-owned YAML 1.2 Core behavior; keep parser types internal and document license/offline/resource implications.
- [ ] Support anchors/aliases and reject multiple documents, duplicate/non-string mapping keys, unsupported tags and nonfinite or unrepresentable JSON values.
- [ ] Keep date-looking and YAML 1.1 boolean-looking values as strings; disclose discarded comments, formatting and alias identity.
- [ ] Bound input, nesting and alias expansion with named policies; refuse over-limit input rather than return partial output.
- [ ] Deliver full Registry/session/Clipboard/History behavior and independent conversion, numeric-fidelity, recursion/alias, Unicode and exact-restore fixtures.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. One YAML 1.2 Core-oriented document with anchors/aliases, strict rejection of multiple documents, duplicate/non-string keys, unsupported tags and nonfinite values, disclosed fidelity loss, and full Registry/session/Clipboard/History behavior. See [`rust/docs/evidence/ticket-13.md`](../../../rust/docs/evidence/ticket-13.md).
