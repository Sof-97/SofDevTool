# Ticket 03 — native Launcher, shortcuts and lifecycle

## Scope

The application opens a Workbench window with a source-defined Registry
(`UtilityRegistry`), a separate Settings window for Launcher shortcut capture,
and a transient nonactivating Launcher panel. Global shortcut registration uses
Carbon `RegisterEventHotKey` behind the app-owned `ShortcutRegistrar` boundary.
The Workbench hides its retained native window on close instead of destroying
it, so Dock reopen restores the same window, selection and child views.

## Reproducible native checks

Host: macOS 26.2 (25C56), Apple Silicon arm64; Rust 1.98.1. Debug bundle
`rust/artifacts/SofDevToolRust.app`, launched outside the checkout with
`SOFDEVTOOL_RUST_SUPPORT_ROOT` pointed at an isolated directory. Inputs were
synthetic (Accessibility `System Events` plus HID-level `CGEvent` clicks).

1. Workbench opens; sidebar, JSON workspace, History panel and footer render.
   See `docs/evidence/previous-migration/assets/03-workbench-history.png`.
2. `Open Utility Launcher` presents a centered nonactivating panel with a
   search field and the two registered Utilities.
3. Typing `diff` filters the list to Text Diff; Enter activates the Workbench
   and opens the Text Diff workspace (embedded Pierre renderer shows café,
   family emoji and rainbow flag).
4. Escape dismisses the Launcher without opening anything.
5. Pressing the registered chord again dismisses the Launcher.
6. A real HID click outside the panel dismisses it (window key-status loss).
7. Invoked from Finder with Control-Option-Space, the Launcher appears over the
   current application while Finder stays the frontmost application (menu bar
   unchanged): the panel is nonactivating.
8. Settings captures Control-Option-K, registers it and persists it under the
   fresh Rust namespace (`launcher-shortcut.v1.json`, `key_code 40`,
   `modifiers 6144`). The old Control-Option-Space default no longer fires and
   Control-Option-K does.
9. An invalid capture (Shift-A, no primary modifier) shows the inline error
   `Use Command, Control, or Option with another key.` and keeps the working
   chord and stored preference unchanged.
10. Closing the last window leaves the process alive with zero visible windows;
    reopening the bundle restores the window with its previous selection
    (Text Diff) and session text intact.
11. With Helium in full screen, Control-Option-K shows the Launcher above the
    full-screen application.

## Verification boundaries

- Click-away is implemented as window key-status loss; it was verified with a
  real HID-level click, not with Accessibility `AXPress` (which does not change
  the key window).
- Shortcut registration failure is covered by the `ShortcutRegistrar` adapter
  test with a fake registrar; no artificial OS-level registration failure was
  forced on the host.
- macOS 14 and 15 runtime remain unverified; the 14.0 deployment baseline is
  unchanged. Only the current display and Space were exercised.

## Automated verification

`CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target rust/scripts/verify --full`
exit 0: rustfmt, Clippy `-D warnings`, 66 tests (30 core, 8 app, 23 JSON
contract, 5 UI), Debug and Release builds. The registry, shortcut controller,
preferences and settings capture have unit tests.
