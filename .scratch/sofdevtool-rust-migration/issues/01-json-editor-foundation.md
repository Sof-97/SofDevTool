# 01: Run JSON in a GPUI Workbench with reusable text controls

Type: task
Status: resolved
Blocked by: None (can start immediately)

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Open a separately identified Rust application, paste JSON, format it and copy the result through controls also demonstrated in an independent component gallery.

## Acceptance criteria

- [x] Establish the three-crate workspace and pinned toolchain/GPUI dependency; document the formatting, build and test gate and a runnable gallery.
- [x] Provide actual JSON format/minify/query controls, neutral empty input and invalid-input diagnostics, using a typed Utility boundary independent of GPUI.
- [ ] Verify multiline selection, undo/redo, Unicode including complex emoji, IME composition, scrolling, explicit Paste/Copy and keyboard focus in the real app.
- [x] Use owner-maintained theme/button/input/editor components in both the application and independently runnable gallery; targeted editor dependencies remain behind the component interface.
- [x] Use distinct Rust identity and storage/preference roots without touching Swift data; do not expose unimplemented Utilities as working.
- [x] Add independent JSON fixtures and editor evidence; record macOS deployment compatibility and actual host runtime separately. History is intentionally introduced in ticket 04.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Implemented and integrated locally in `9b5703302dd1134955804b2a4ca8a4d9ef775d13`
on `feat/rust-gpui-migration`, including GPT-5.6 Terra fixes for native Unicode
deletion and gallery clipping. Standards and Spec re-reviews have no remaining
source findings. The full Rust gate passes (35 tests, Clippy, Debug/Release).

[Native evidence](../evidence/2026-09-21-native-resume.md) records real editing,
Clipboard, selection, undo/caret, scrolling, Query, focus and gallery observations
on macOS 26.2. IME composition/commit/cancel remains unverified. On 2026-09-22 the owner
explicitly rejected Japanese input setup as a blocker and directed implementation
of all tickets to continue. Ticket 01 is accepted with this documented native
evidence exception; no IME pass is claimed and no OS input source is changed.
macOS 14/15 runtime remains unverified. Tickets 02/03 are unblocked.
