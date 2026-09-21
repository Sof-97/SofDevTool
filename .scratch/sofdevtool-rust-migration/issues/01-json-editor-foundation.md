# 01: Run JSON in a GPUI Workbench with reusable text controls

Type: task
Status: ready-for-agent
Blocked by: None (can start immediately)

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Open a separately identified Rust application, paste JSON, format it and copy the result through controls also demonstrated in an independent component gallery.

## Acceptance criteria

- [ ] Establish the three-crate workspace and pinned toolchain/GPUI dependency; document the formatting, build and test gate and a runnable gallery.
- [ ] Provide actual JSON format/minify/query controls, neutral empty input and invalid-input diagnostics, using a typed Utility boundary independent of GPUI.
- [ ] Verify multiline selection, undo/redo, Unicode including complex emoji, IME composition, scrolling, explicit Paste/Copy and keyboard focus in the real app.
- [ ] Use owner-maintained theme/button/input/editor components in both the application and independently runnable gallery; targeted editor dependencies remain behind the component interface.
- [ ] Use distinct Rust identity and storage/preference roots without touching Swift data; do not expose unimplemented Utilities as working.
- [ ] Add independent JSON fixtures and editor evidence; record macOS deployment compatibility and actual host runtime separately. History is intentionally introduced in ticket 04.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Pending implementation. The parent specification supplies shared behavior; this ticket makes no completion or runtime claim.
