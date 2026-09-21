# 02: Compare text in an embedded web renderer inside GPUI

Type: task
Status: claimed
Blocked by: 01

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Open a minimal Text Diff workspace in the same application and compare Unicode text using locally bundled web rendering.

## Acceptance criteria

- [ ] Render split and unified examples in a real embedded WebView alongside GPUI controls, not a separate browser or web shell.
- [ ] Verify resizing, clipping, display scaling, scroll, selection, Copy and focus transfer between GPUI input and the web surface.
- [ ] Demonstrate offline resources, UTF-8 bridge correctness, complex-emoji handling and visible loading/error states with idempotent readiness.
- [ ] Keep native/web types behind the application Text Diff renderer boundary and document the selected integration, license inventory and remaining limitations.
- [ ] Record reproducible real-app evidence. A failed integration remains unresolved and blocks broad migration; do not silently replace the renderer approach.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Pending implementation. The parent specification supplies shared behavior; this ticket makes no completion or runtime claim.
