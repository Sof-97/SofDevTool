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

## Coordinator integration and native checkpoint

Integrated candidate `ae00b698c4726246c8417a81124ac39bfabdb063`. A frozen Git archive passed formatting, strict Clippy and 323 Rust tests (90 app, 190 core, 23 JSON fixtures, 9 retained vectors, 11 sofui). The archive lacks Git metadata, so its packaging metadata test deliberately refused it; this is not a complete `make verify` pass. Live checkout packaging remains covered by ticket 20/21 and the final gate. The native binary was built from the exact archive and matched SHA-256 `60e3ff69bc3a3e8ddf51a9bc9488314577d129c88b4d68e981948cec9f85fdc0`.

The temporary bundle `/private/tmp/sofdevtool-wave7-vdnjy4ol/Native Shell.app` ran from `/private/tmp` with `SOFDEVTOOL_RUST_SUPPORT_ROOT=/private/tmp/sofdevtool-wave7-profile-dbq7rf4q`, after explicit owner approval of native launch and the terminal launch carrying that variable. The initial Computer Use launch without that override was closed before any Utility interaction. All following observations used the verified isolated process. Host: macOS 26.2 arm64.

Observed via Computer Use:

- JSON input `{"native":"caffè 👩🏽‍💻","value":7}` formatted correctly and recorded one completed result. Switching to Frappé, Base64 and back retained exact input/result and that single History entry.
- Favorites filtered to the saved JSON entry; Recent showed JSON and Base64 without disturbing the current workspace.
- Launcher search `diff` plus Return opened Text Diff. Escape dismissed the Launcher. Fourteen Down presses scrolled the selected final YAML/JSON row into view.
- The native full-screen button entered fullscreen and the Launcher remained usable over that Workbench Space. This is not a cross-display or another application's fullscreen acceptance claim.
- Settings showed a diagnostic for plain `k` during shortcut capture, then accepted Ctrl-Option-K and updated the Workbench shortcut label. Actual invocation with CUA's synthesized chord did not show the Launcher; native global invocation remains unresolved, with no assumption whether this is an event-injection limit or application defect.
- Cmd-Q exited from fullscreen and process inspection confirmed termination.

The direct run exposed missing Cmd-W / File Close behavior and missing keyboard fullscreen toggling. A separate lifecycle correction is assigned before final acceptance. Last-window/Dock reopen, cross-application nonactivation, final Utility redesign fit and installed Release acceptance remain for the corrected final candidate. Screenshots are in the coordinator conversation.

### Additional isolated relaunch and retained-window check

A second explicitly authorized terminal launch of the same bundle/profile
preserved the saved Frappé theme, favorite JSON, configured Ctrl-Option-K and
the single recorded JSON operation. Workspace input/result started empty after
a process relaunch, as intended. Restoring that History entry populated the
exact input/result without adding another record. Native close then hid the
last window while PID 70160 stayed alive. Reopening through Computer Use's
application-open API restored the same process/window with exact input, result
and selected History retained. This exercises native reopen behavior; a literal
Dock icon click was not performed (the Dock accessibility target timed out).
Cmd-Q then terminated the process, confirmed by process inspection.

The synthetic Ctrl-Option-K chord also failed to expose a Launcher when sent
while the separate preview application was foreground. Global hardware-key
invocation therefore remains unverified; no Carbon implementation change has
been made on this evidence alone.

## Final owner-reported shell acceptance — 2026-09-23

The coordinator later installed the corrected r4 Release 0.2.0 build 1,
packaged as clean revision `b4e17030ffa8e37b51bb661befe2e6728a044b9b`
with executable SHA-256
`24d2645b47021748226be8f3919e9fe1bc062730f92b710a7955fbcfe5e2a905`.
The app ran outside the checkout against isolated
`/private/tmp/sofdevtool-final-profile-0jm89bq9`; the latest owner handoff
used PID 14600. The coordinator had restored the synthetic JSON
`items[1].name` query and `caffè` result and left the app open for manual
checks. The installed version, bundle identity and executable hash were
reverified before closeout.

The owner was specifically asked to test the physical Control-Option-Space
Launcher shortcut from another application, interaction from another
Space/full-screen context, and Cmd-W followed by a literal Dock click returning
to the retained JSON state. The owner replied “Si sembra funzionare tutto.”
This is an **owner-reported pass of those requested checks**, not a
Computer Use observation of the key, Space or Dock actions. Earlier CUA
synthetic-key failures and its Dock automation timeout remain historically
accurate; they do not override the owner's manual result. Other shell,
Settings, theme, editing and WebView interactions are separately recorded in
[ticket 22](ticket-22.md). Native runtime on macOS 14/15, cross-display/DPI
transitions and full IME composition were not established on this macOS 26.2
arm64 host.
