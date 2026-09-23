# Ticket 22 — packaged release acceptance

## Utilities and shell

All fifteen Utilities are implemented, registered and reachable from the
Workbench and Launcher: JSON, YAML/JSON, Base64, URL Encoding, Hashes,
Identifier Generator (UUID/ULID/KSUID), Timestamps, JWT Decoder, Regex, Case
Conversion, Whitespace, Color Conversion, Sample Data, Random String and Text
Diff. No catalog placeholders remain. Each Utility has Registry metadata, a
retained session, diagnostics, explicit Clipboard actions, versioned snapshots,
History preview/restore and domain tests.

## Packaging

`rust/scripts/bundle-release` builds a Release bundle with a distinct identity
and intentional version/build identity:

- `CFBundleIdentifier com.gerardo.sofdevtool.rust` (distinct from the Swift
  `dev.gerardo.SofDevTool`), `LSMinimumSystemVersion 14.0`.
- `CFBundleShortVersionString 0.1.0`, `CFBundleVersion 1`.
- A locally generated `AppIcon.icns` (no network or external assets).
- `THIRD_PARTY_NOTICES.md` copied into `Contents/Resources` (it is also compiled
  into the binary and shown in the Text Diff workspace).

`plutil -lint` passes. WebView assets are embedded via `include_str!`, so the
bundle needs no external resources.

## Outside the checkout, offline, fresh start and persistence

Host: macOS 26.2 (25C56), Apple Silicon arm64; Rust 1.98.1.

1. The Release bundle was copied to `/private/tmp/SofDevToolRustRelease.app` and
   launched outside the source checkout with an isolated
   `SOFDEVTOOL_RUST_SUPPORT_ROOT`. First run showed Graphite, empty History
   (`0/25`), no favorites and no recents.
2. Pasting a JSON object created `History/json.history.v1.json` and recorded
   `1/25`.
3. Relaunching the same bundle reloaded the entry (`1/25`) and its preview:
   fresh Rust data persists without reading legacy data.
4. The Swift Application Support namespace (`~/Library/Application
   Support/SofDevTool/History`) and the `dev.gerardo.SofDevTool` preference
   domain were left untouched; the Rust app writes only under `SofDevToolRust`.

Offline containment is structural: assets are embedded, the renderer's CSP
rejects remote subresources and navigation is restricted, and no Utility makes a
network request. No packet capture or OS network-setting change was performed.

## Automated gates

`CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target rust/scripts/verify --full`
exit 0: rustfmt, Clippy `-D warnings`, 257 tests (43 app, 186 core, 23 JSON
contract, 5 UI), and Debug + Release builds of every binary and example. The
gallery example builds without the application crate.

## Native scenarios run on the host

- Editor: multiline Unicode editing, grapheme deletion, undo, selection and the
  renderer focus handoff (tickets 01/02/21).
- Launcher: global shortcut registration and capture, nonactivating presentation
  over the active application and a full-screen app, keyboard search/selection,
  Escape/click-away/repeated-shortcut dismissal, Dock reopen and Quit (ticket 03).
- WebView: split/unified rendering, selection/copy, resize, offline assets and
  error/recovery (tickets 02/21).
- History: recording, relaunch persistence, exact restore and the Clear hold
  (tickets 04/05/06/21).
- Catalog: Library/Recent/Favorites scopes, search, grouping, favorites/recents
  persistence, theme switching and session preservation (ticket 07).
- Keyboard/focus: visible focus and Enter/Space activation across workspaces.

## Untested and deferred

- **macOS 14 and 15 runtime**: not exercised. Only macOS 26.2 arm64 was run; the
  14.0 deployment baseline is unchanged.
- IME composition remains the previously recorded nonblocking evidence
  exception (no OS input source was changed).
- Cross-display/DPI transitions and a screen-reader audit were not exercised;
  full VoiceOver parity is deferred by the specification.
- No network capture was performed to prove offline operation beyond the
  structural CSP/asset evidence.
