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
   **Owner decision requested:** accept this narrow app-owned composition, or
   file the deletion behavior upstream and treat it as a completion blocker per
   the specification. This is reported, not silently kept.
2. **Modal dialogs require a host-rendered layer.** `gpui_kit::component::Root::render`
   does not mount `render_dialog_layer`; a view that opens dialogs must include
   `Root::render_dialog_layer(window, cx)` in its own tree. The Workbench and
   the Settings window do. This is recorded so future surfaces do not miss it.
3. **Programmatic catalog focus** uses `crate::ui::button_focus`, which reads
   the focus handle GPUI Kit keys by a button's element id. The handles are
   built during `Workbench::render`, where `Window::use_keyed_state` is legal;
   it must not be called outside layout/prepaint/paint. Only the Workbench
   catalog arrow navigation uses it.

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
