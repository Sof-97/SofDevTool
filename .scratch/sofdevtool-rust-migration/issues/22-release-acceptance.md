# 22: Package and verify the complete Rust Developer Toolbox

Type: task
Status: resolved
Blocked by: 05, 07, 08, 09, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Launch the complete Rust application from a normal macOS bundle, use all fifteen Utilities offline and retain new Rust settings/History without affecting Swift.

## Acceptance criteria

- [ ] Verify every parent-spec Utility and shared shell/History criterion; no visible catalog placeholders or missing restore flows remain.
- [ ] Package Debug/Release with distinct identity and appropriate icons, intentional version/build identity, local WebView resources and third-party notices.
- [ ] Run outside the checkout with networking unavailable; verify asset resolution, first-run isolation, subsequent persistence and no legacy data changes.
- [ ] Run authoritative Rust automated/release gates and record real editor/Launcher/WebView/keyboard/Dock scenarios on the actual host; list untested macOS versions explicitly.
- [ ] Verify the component gallery builds without app dependencies and document component consumption, test commands, module ownership and deferred repository extraction.
- [ ] Provide owner installation/launch instructions and preserve Swift fallback/source/data. Replacing an installed Swift app or deleting it requires a separate owner action.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. All fifteen Utilities and the shared shell/History behavior are verified; the Release bundle has a distinct identity, icon, version and bundled notices, runs outside the checkout with fresh-then-persistent Rust data and leaves Swift data untouched. Native scenarios, gate results and the explicit untested list are in [`rust/docs/evidence/ticket-22.md`](../../../rust/docs/evidence/ticket-22.md). macOS 14/15 runtime remain unverified.
