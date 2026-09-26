# 10: Native acceptance record

**Status:** awaiting-native-acceptance
**Type:** task
**Recorded:** 2026-09-26
**Implementation revision:** working tree based on `e67d3fdcef011a9e9ad39ede062c74b3834e7521` (source reported dirty by the packaging metadata)
**Host used for automated checks:** macOS 26.2 (build 25C56), arm64

This record separates what was actually exercised from what was not. It does
**not** claim native acceptance. No native desktop session, no mouse/keyboard
interaction and no visible-window observation were performed by the
implementing agent; only builds and automated tests were run.

## Automated evidence (observed)

`make verify-full` completed with `gate passed` and exit 0 on the host above.
It ran, in order:

- `cargo fmt --all --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test --workspace` — all tests pass, including 107 `sofdevtool-app`
  library tests, the `sofdevtool-core` suites and the retained contract
  vectors.
- `python3 -m unittest discover -s scripts/rust -p 'test_*.py'` — 5 packaging
  and temporary-install tests pass.
- `cargo build --workspace --bins --examples` (Debug) and
  `cargo build --release --workspace --bins --examples` (Release) — both
  finish.
- `cd crates/app/src/text_diff/assets-source && npm ci && npm run verify` —
  "Text Diff bundle, dependency inventory and notices are reproducible."
- `scripts/rust/package.py --profile debug` and `--profile release` — both
  bundles are produced under `artifacts/`.
- the identity test passes against both bundles.

Target controlled by the host at implementation time: none of this is native
runtime evidence. A build, an automated GPUI test, a temporary install or a run
on macOS 26.2 does not establish macOS 14/15 behavior.

## Coverage added for previously untested behavior

- `crates/app/src/ui.rs` unit tests cover the retained whole-grapheme deletion
  rules (ZWJ family, base+combining, partial selection, UTF-16 offsets).
- `crates/app/src/utilities/base64.rs` now drives a real kit multiline editor:
  it types `cafe\u{301}` and asserts that one Backspace removes the whole
  `e\u{301}` grapheme, then restores the first History entry and asserts the
  entry count is unchanged (restore records nothing).

## Upstream findings and owner decisions

1. **Whole-grapheme deletion is not in GPUI Kit / gpui-base 0.6.6.** The kit
   editor's Backspace/Delete remove one Unicode scalar
   (`InputBaseState::previous_boundary`/`next_boundary` handle only `\r\n`), so
   a ZWJ emoji family or base+combining sequence would be split. The Toolbox
   preserved whole-grapheme deletion before this migration. It is preserved
   now by the app-owned `crate::ui::multiline_editor` composition, which
   intercepts the two kit actions around the kit `Textarea`; it is covered by
   the tests above and documented in `crates/app/src/ui.rs`. The kit control
   still owns rendering, selection, IME, undo/redo and scrolling.
   **Owner decision — approved 2026-09-26:** “Ok allora niente dobbiamo
   tenerci la nostra soluzione”. Retain the existing narrow adapter, including
   expansion of partial selections to whole graphemes. Standards P2 ownership
   conflict is resolved by this explicit exception; native acceptance and the
   overall migration remain open. Revisit removal only with verified equivalent
   upstream behavior.

   The experiment on `f07917cad3d5a71f5788ae62eeb19c95d868bde9` removed the
   adapter without changing the Base64 interaction assertion. Backspace on
   `cafe\u{301}` produced `cafe`, while the retained requirement expects `caf`.
   `make verify` failed on that assertion (107 app tests passed, one failed).
   Logs: `/tmp/sofdevtool-kit-deletion-probe.log` and
   `/tmp/sofdevtool-kit-deletion-verify.log` (temporary). The experimental Debug
   bundle was opened with isolated data; this is not coordinator-observed native
   acceptance. [Upstream research](../upstream-unicode-research.md) found no
   ready-to-adopt fix in release 0.6.6 or the inspected main revision.
2. **Modal dialogs require a host-rendered layer.** `gpui_kit::component::Root::render`
   does not mount `render_dialog_layer`; a view that opens dialogs must include
   `Root::render_dialog_layer(window, cx)` in its own tree. The Workbench and
   the Settings window do. This is recorded so future surfaces do not miss it.
3. **Programmatic catalog focus** uses `crate::ui::button_focus`, which reads
   the focus handle GPUI Kit keys by a button's element id. The handles are
   built during `Workbench::render`, where `Window::use_keyed_state` is legal;
   it must not be called outside layout/prepaint/paint. Only the Workbench
   catalog arrow navigation uses it.

## Comments

### 2026-09-26 — Standards P2 resolution validation

Restored the existing adapter and its five unit tests after the kit-only
experiment; its implementation is unchanged from `f07917c`. Updated AGENTS,
specification, map and source comments to reflect the owner's approval above.
`make verify` passed on the restored working tree: 335 Rust tests (113 app),
formatting, Clippy, five packaging/install tests and Debug build. Log:
`/tmp/sofdevtool-grapheme-restored-verify.log` (temporary).
No new coordinator-observed native acceptance was performed, and this does not
close ticket 10 or accept the whole migration. The Debug bundle was also rebuilt
with the restored adapter; package log: `/tmp/sofdevtool-grapheme-restored-package.log`.

## Compatibility

`gpui-kit 0.6.6` resolves `gpui` (`gpui-pre`) to `=0.3.6` and `gpui-component`
to `^0.6.6`, matching the versions the application already pinned. This was an
adoption and removal effort, not a version bump; no new behavior beyond the
findings above was observed.

## Required native acceptance (not done)

In an isolated profile, without overwriting the installed app or personal data
(`SOFDEVTOOL_RUST_SUPPORT_ROOT`, `INSTALL_DESTINATION`), exercise and record:

- all fifteen Utilities: representative input/result, changed controls, session
  switching, completed-operation History capture and restore;
- System/Light/Dark, live macOS appearance change, both legacy preference
  values (`graphite`, `catppuccin`) and restart persistence with Debug/Release
  isolation;
- Launcher activation, window reopening, keyboard navigation, Unicode editing,
  selection, undo/redo, explicit Clipboard actions and destructive dialog
  cancellation/confirmation;
- Text Diff appearance, focus, resize, restore and recovery in the packaged app
  offline.

Record OS/version, build revision and direct observations. Missing evidence is
missing, not passing.
