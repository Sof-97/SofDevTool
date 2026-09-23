# Launcher Carbon hotkey press-event correction

The preceding installed Release's synthesized Ctrl-Alt-Space did not open the Launcher. A read-only comparison with the local macOS SDK found a concrete source mismatch: `crates/app/src/shortcut.rs` named its installed `EventTypeSpec` as `kEventHotKeyPressed` but used event kind `6`. The pinned host SDK's `HIToolbox/CarbonEvents.h` defines `kEventHotKeyPressed = 5` and `kEventHotKeyReleased = 6`. The app had therefore subscribed to release rather than press delivery. This does not by itself prove how a physical chord behaved or whether CUA generated a Carbon release event.

The macOS registrar now uses named `KEYBOARD_EVENT_CLASS` and `HOT_KEY_PRESSED_EVENT` constants, with the SDK correspondence documented beside them. The `RegisterEventHotKey` keycode/modifier/options, callback flag, Workbench observer, Launcher presentation and failure diagnostics remain unchanged. The registrar still returns an error when Carbon registration fails. The SDK permits the same nonexclusive chord in multiple processes; both Debug and Release showing the default shortcut in Settings is expected and does not prove delivery. Settings displays the configured default even when no active registration exists, while the Workbench error banner is the registration-failure signal.

The current app has one Carbon hotkey registration path. Its callback does not inspect the delivered hotkey ID; with no second registration in this process, no additional current misrouting was found. This was not expanded into a new hotkey feature. A unit test that merely asserted the new numeric constant would mirror the implementation, so verification uses the authoritative SDK mapping, existing shortcut controller tests, strict app compilation and a separate corrected native run. Astra's follow-up checked SDK lines 1339, 4693 and 4715, accepted the Pressed `5` correction and transactional registration, and reported no further regression. **Physical native press delivery remains unverified.**

## Verification boundary

- A read-only comparison against the local SDK's `CarbonEvents.h` confirmed Pressed `5`, Released `6`, and app subscription `5`. The SDK header uses a non-UTF-8 byte; the check decoded it as Latin-1 after a first UTF-8 read attempt failed. No source or SDK file was changed by this check.
- `rustfmt --edition 2021 --check crates/app/src/shortcut.rs` passed.
- With `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target` and `CARGO_NET_OFFLINE=true`, `cargo test -p sofdevtool-app --lib shortcut::tests -- --nocapture` passed all five controller tests. These tests validate configuration and transactional registration but cannot inject a real Carbon hotkey event.
- `cargo clippy -p sofdevtool-app --all-targets -- -D warnings` passed. The pre-existing transitive `block 0.1.6` future-incompatibility notice remains separate from first-party Clippy.

The final [r4 full gate](../../../.artifacts/parallel-rust-only/final/verify-full-r4.log) passed 339 Rust tests, five Python checks and identity. The corrected Release `b4e17030ffa8e37b51bb661befe2e6728a044b9b` was installed at the default location; the Launcher button opened the native panel and Escape closed it. CUA-synthesized Ctrl-Alt-Space **still did not open it**. This is a post-fix synthetic observation, not a physical-key test or proof of a remaining Carbon defect. A Settings shortcut label also does not establish delivery.

The coordinator subsequently asked the owner to press the physical
Control-Option-Space chord from another application on this installed r4
Release, along with separate Space/full-screen and Dock-return checks. The
owner replied “Si sembra funzionare tutto.” This is an **owner-reported pass
of the physical chord**, not a CUA-captured Carbon event trace; the synthetic
failure above remains accurately described as an automation observation.
See [ticket 22](ticket-22.md) for the exact installed binary, isolated profile
and manual-check provenance. macOS 14/15 and other keyboard layouts were not
tested.
