# 19: Convert and select sRGB colors

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Enter a supported CSS color or select one interactively, inspect synchronized representations and restore the captured color.

## Acceptance criteria

- [ ] Support bounded sRGB HEX/RGB(A)/HSL(A), alpha and deterministic rounding while rejecting unsupported color spaces/syntax.
- [ ] Provide a usable color-selection control synchronized with text and preview; invalid typed text must not silently become a different accepted color.
- [ ] Add reusable color controls to the UI library/gallery only where they have no SofDevTool-specific dependencies.
- [ ] Deliver full Registry/session/Clipboard/snapshot behavior with boundary, alpha, round-trip and interaction tests.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. See [`rust/docs/evidence/ticket-19.md`](../../../rust/docs/evidence/ticket-19.md) for the gate results, native smoke checks and limits (macOS 14/15 runtime remain unverified).
