# 02: Compare text in an embedded web renderer inside GPUI

Type: task
Status: resolved
Blocked by: 01

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Open a minimal Text Diff workspace in the same application and compare Unicode text using locally bundled web rendering.

## Acceptance criteria

- [x] Render split and unified examples in a real embedded WebView alongside GPUI controls, not a separate browser or web shell.
- [x] Verify resizing, clipping, display scaling, scroll, selection, Copy and focus transfer between GPUI input and the web surface.
- [x] Demonstrate offline resources, UTF-8 bridge correctness, complex-emoji handling and visible loading/error states with idempotent readiness.
- [x] Keep native/web types behind the application Text Diff renderer boundary and document the selected integration, license inventory and remaining limitations.
- [x] Record reproducible real-app evidence. A failed integration remains unresolved and blocks broad migration; do not silently replace the renderer approach.
- [x] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Implemented and verified on macOS26.2 arm64. See [native/build evidence](../../../rust/docs/evidence/ticket-02.md) and the [operational register](../operations.md). Current-display scaling was observed; cross-display transitions and macOS14/15 runtime remain unverified. Local assets, CSP and navigation restrictions provide source evidence for offline containment; no network capture was performed.
