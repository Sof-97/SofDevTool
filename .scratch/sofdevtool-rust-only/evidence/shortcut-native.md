# Launcher Carbon hotkey press-event correction

The installed Release's synthesized Ctrl-Alt-Space did not open the Launcher. A read-only comparison with the local macOS SDK found a concrete source mismatch: `crates/app/src/shortcut.rs` named its installed `EventTypeSpec` as `kEventHotKeyPressed` but used event kind `6`. The pinned host SDK's `HIToolbox/CarbonEvents.h` defines `kEventHotKeyPressed = 5` and `kEventHotKeyReleased = 6`. The app had therefore subscribed to release rather than press delivery. This does not by itself prove how a physical chord behaved or whether CUA generated a Carbon release event.

The macOS registrar now uses named `KEYBOARD_EVENT_CLASS` and `HOT_KEY_PRESSED_EVENT` constants, with the SDK correspondence documented beside them. The `RegisterEventHotKey` keycode/modifier/options, callback flag, Workbench observer, Launcher presentation and failure diagnostics remain unchanged. The registrar still returns an error when Carbon registration fails. The SDK permits the same nonexclusive chord in multiple processes; both Debug and Release showing the default shortcut in Settings is expected and does not prove delivery. Settings displays the configured default even when no active registration exists, while the Workbench error banner is the registration-failure signal.

The current app has one Carbon hotkey registration path. Its callback does not inspect the delivered hotkey ID; with no second registration in this process, no additional current misrouting was found. This was not expanded into a new hotkey feature. A unit test that merely asserted the new numeric constant would mirror the implementation, so verification uses the authoritative SDK mapping, existing shortcut controller tests, strict app compilation and a separate corrected native run. **Native press delivery remains pending the coordinator's installed-Release check.**

## Verification boundary

- A read-only comparison against the local SDK's `CarbonEvents.h` confirmed Pressed `5`, Released `6`, and app subscription `5`. The SDK header uses a non-UTF-8 byte; the check decoded it as Latin-1 after a first UTF-8 read attempt failed. No source or SDK file was changed by this check.
- `rustfmt --edition 2021 --check crates/app/src/shortcut.rs` passed.
- With `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target` and `CARGO_NET_OFFLINE=true`, `cargo test -p sofdevtool-app --lib shortcut::tests -- --nocapture` passed all five controller tests. These tests validate configuration and transactional registration but cannot inject a real Carbon hotkey event.
- `cargo clippy -p sofdevtool-app --all-targets -- -D warnings` passed. The pre-existing transitive `block 0.1.6` future-incompatibility notice remains separate from first-party Clippy.

The coordinator owns a fresh installed-Release native press check and any final full gate/review. The pre-fix CUA failure and a Settings shortcut label are not post-fix delivery evidence.
