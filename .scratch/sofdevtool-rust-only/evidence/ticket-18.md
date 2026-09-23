# Ticket 18 — Text Diff workspace redesign

The Text Diff workspace now uses the approved compact controls while retaining
its application-owned Wry/WebKit renderer boundary and exact UTF-8 revision
protocol. The bundled assets and editable asset pipeline from ticket 17 are
unchanged.

## Implementation

- Split and Unified are a retained-focus `SegmentedControl` with stable option
  IDs. Explicit Paste, Copy and History actions have stable Button IDs.
- The trailing inspector uses generic `SelectableList` and `ConfirmationBar`.
  The application still supplies History wording, selection, availability,
  retention count, exact snapshot decoding and restore decisions.
- Original and Updated editors use the compact, labelled panel convention. A
  bounded Comparison panel gives the native child WebView the remaining flex
  area; toolbar controls wrap at narrower widths. Status, complex-emoji
  fallback and failure diagnostics remain visible below the comparison.
- The existing session remains mounted when another Utility is selected.
  Restore still replaces exact old/new text and mode without recording a new
  operation. Renderer readiness, stale callback rejection and recovery code
  are unchanged.

## Verification

- Scoped `rustfmt --edition 2021` on `crates/app/src/text_diff/mod.rs` — passed.
- A source search found no `HistoryPanel`, `HistoryItem`, or `Button::new`
  compatibility calls in the Text Diff workspace.
- Added a GPUI interaction test for keyboard Split/Unified selection, explicit
  Copy of Unicode sample text, generic History selection and confirmed exact
  Unicode/mode restore without rerecording. The coordinator's combined
  `make verify` compiled this test and it passed; 101 of 102 application
  tests passed, with the sole failure in ticket 14's Hashes empty-Copy test.
  The combined gate stopped there, so this is not a full-gate pass.

Native resize/clipping, selection, scrolling, WebView Copy, focus handoff and
final redesigned Release acceptance remain coordinator-owned. No native app
was launched by this worker for ticket 18.

## Native checkpoint — 2026-09-23

Coordinator ran a frozen Debug bundle assembled from `e3cb13b89193f9768d87183f1efd9e6220ac52f8` on macOS 26.2 arm64, outside the checkout with the explicitly isolated profile `/private/tmp/sofdevtool-wave7-profile-dbq7rf4q`. Bundle executable SHA-256: `2c4afd2aaa60addc547c36782402b634e70dc662836383de25ad281051ba81bb`.

- Entered original/updated `caffè 👩🏽‍💻 🇮🇹 1️⃣` lines and different second lines. Split and Unified render correctly in the bounded Comparison area, with the complex-emoji fallback disclosure and generic History entries.
- Double-clicked the rendered word `caffè`; native Edit > Copy followed by clicking the GPUI Updated editor and Cmd-V pasted exactly `caffè`. This establishes native selection, menu Copy and return to the editor.
- Two synthesized Cmd-C attempts from the same WebView selection yielded empty paste. This remains a keyboard-dispatch/CUA limitation under diagnosis; keyboard Copy is not accepted from this run.
- Standard AppKit View > Enter/Exit Full Screen worked and preserved the workspace. Custom lifecycle commands from the same candidate did not; a separate correction is in progress.

This is an intermediate Debug checkpoint, not final installed Release acceptance. Native scrolling/resizing and final Release checks remain pending.
