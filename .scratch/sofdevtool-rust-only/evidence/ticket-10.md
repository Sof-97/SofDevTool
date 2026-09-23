# Ticket 10: observable application-wide themes

Implementation candidate, 2026-09-23. Host: macOS 26.2 (25C56), arm64;
Rust 1.98.1. The coordinator owns commit integration and native launch.

## Behavior

- `sofui::apply_theme(variant, cx)` selects Graphite or Catppuccin Frappé;
  `apply_custom_theme(tokens, cx)` accepts public `ThemePalette` RGB semantic
  values. Both update the shared component snapshot, the wrapped editor theme,
  and GPUI's app-wide window refresh queue. They operate on existing entities;
  no workspace/editor reconstruction or user-edit event is involved. Legacy
  theme accessors remain for consumers until ticket 19.
- The editor bridge updates foreground, input/editor background, border, caret,
  selection, focus, diagnostic and related legacy tokens, then synchronizes the
  dependency's base/text defaults. Its syntax theme editor background is
  explicitly mapped so it cannot mask the selected palette.
- Workbench initializes from its existing app-owned workspace preference and
  persists preset changes there. Graphite remains the fresh default. The app
  entrypoint seeds Graphite; Launcher and Settings inherit the current global
  palette and receive the same window refresh. No application History, Recent,
  Utility input or result state is written by theme application.
- The gallery offers both presets and a distinct custom palette through public
  sofui controls. The library README documents the setup, customization and
  persistence boundary.

## Verification

All Cargo commands use `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target`
and `CARGO_NET_OFFLINE=true` from `rust/`.

| Command | Result |
| --- | --- |
| `cargo check -p sofui --all-targets -p sofdevtool-app` | Passed after initial theme bridge; final `cargo check -p sofdevtool-app` also passed after all theme changes. |
| `cargo test -p sofui --test public_themes` | Passed: two retained windows and a third opened after customization render the selected palettes; wrapped editor globals, unchanged Unicode content/change counts and retained focus are checked. |
| `cargo test -p sofui` | Passed: 5 editor, 2 component interaction, and 1 public theme test. |
| `cargo clippy -p sofui --all-targets -- -D warnings` | Passed. |
| `cargo test -p sofdevtool-app` | Blocked during concurrent ticket 02 edits by a test-compilation error in `settings.rs:743` (`weak.update` closure type inference). No ticket 10 app test failure was reported; rerun after ticket 02 stabilizes. |
| `rustfmt --edition 2021 --check` on owned Rust files | Passed. |

The public test draws synthetic GPUI windows and checks editor theme values.
It does not establish native appearance for the actual Workbench, Launcher or
Settings, nor macOS 14/15 runtime behavior. The coordinator's native gate must
observe those surfaces, including focus and session continuity, separately.

Coordinator integration note: ticket02 subsequently corrected its temporary
test compilation error and all 68 application tests passed on the combined
candidate. The shared default gate found one separate Clippy style diagnostic
in History retry, routed to its owner before integration.
