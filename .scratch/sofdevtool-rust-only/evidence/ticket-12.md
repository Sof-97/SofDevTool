# Ticket 12 — approved shell design rollout candidate

The owner approved the ticket 11 native candidate at `b7e28f0`. This slice
applies its compact hierarchy to the real Workbench, Utility Launcher and
separate Settings window. Utility-specific bodies remain assigned to tickets
13–16 and 18; this slice does not change their domain state or History policy.

## Changed shell behavior and presentation

- Workbench: 54-pixel toolbar with Launcher, Settings and both live theme
  presets; 252-pixel Library/Recent/Favorites sidebar with grouped, searchable
  15-Utility catalog; 13-pixel root typography and restrained borders. The
  selected Utility has one title/summary above its retained workspace entity.
  The old outer panel was removed, giving editors and the native Text Diff
  child view more room without reconstructing their sessions. Utility-owned
  trailing History remains in each workspace. Up/Down focus visible catalog
  rows and scroll them into view; Enter/Space uses the existing sofui Button
  action. Recents, Favorites and theme persistence still use the same
  application-owned state. The Launcher button displays the current registered
  shortcut rather than a stale default when the owner customizes it.
- Launcher: the existing nonactivating native panel, active-display placement,
  dismissal/opening actions and search selection model are retained. The
  compact searchable result list scrolls the selected row into view, shows its
  source category, and provides a neutral no-results state. Escape, Up/Down
  and Return keep their existing scoped action bindings.
- Settings: the separate resizable window uses compact shortcut and History
  cards. Shortcut capture, invalid/failed registration diagnostics, recording
  toggles, per-Utility holds/retry, unknown stored files, and Clear All retain
  their existing callbacks and IDs. Confirmation is placed above the long
  Utility list so keyboard/assistive requests remain visible. Pointer holds
  still use one and two seconds; keyboard activation still asks for ordinary
  confirmation.

No sofui API, Utility workspace, application lifecycle, manifest or product
storage code changed in this slice. Shared visual conventions for Utility
rollout: `ThemeTokens::active()`, 13-pixel root text, small muted captions,
8–12-pixel control gaps, existing sofui Button/panel/diagnostic/hold/confirmation
surfaces, and a single title hierarchy. Utility content continues to own its
History inspector and retained session.

## Checks completed

- `rustfmt --edition 2021 crates/app/src/workbench.rs crates/app/src/launcher.rs crates/app/src/settings.rs` — passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo test -p sofdevtool-app --lib` — 90 passed, 0 failed after the final shell edits. Includes Registry/opening, isolated Settings/History/shortcut and workspace regression tests.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo clippy -p sofdevtool-app --all-targets -- -D warnings` — passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo build -p sofdevtool-app --bin sofdevtool` — passed.

The worker did not launch the native app. The coordinator owns direct native
checks of Launcher active-Space/full-screen placement, nonactivating dismissal,
registered-shortcut failure/recovery, Utility opening and session continuity,
focus and WebView handoff, Settings, Dock reopen, last-window close and explicit
Quit. A build and tests do not prove those macOS interactions or visual fit.
