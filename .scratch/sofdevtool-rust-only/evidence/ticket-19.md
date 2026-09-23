# Ticket 19 — sofui extraction readiness

`sofui` is now a separately buildable 0.1.0 component package with explicit
edition, license, dependency versions and dedicated lockfile. Its public
interfaces contain presentation and interaction logic only; the application
owns domain behavior, Clipboard policy, History, preferences and lifecycle.

## Compatibility contraction

- Removed the product-specific `HistoryPanel`/`HistoryItem` adapter and its
  export after all fifteen Utility workspaces adopted generic
  `SelectableList`/`ConfirmationBar`.
- Buttons now require caller-owned stable IDs. Removed the label-derived
  `Button::new`/`primary`/`id` compatibility path and converted the remaining
  Settings controls. A redundant index-based ID override in Sample Data was
  removed, retaining its stable field ID.
- Text controls expose explicit `assign_text` (silent initialization, restore,
  derived result) and `edit_text` (undoable user-style change) methods; removed
  the old `set_text` and `replace_all` aliases after migrating application
  consumers with the same call semantics. The app imports `sofui` directly,
  without the temporary `sofdevtool-ui` Cargo alias.
- Removed the temporary `workbench_preview` example and `--text-diff-proof`
  startup option after the approved design and native renderer proof were
  integrated. The ordinary application startup, Utility catalog and renderer
  implementation remain in place.
- Removed non-observable theme compatibility setters from sofui's public API;
  `apply_theme` and `apply_custom_theme` remain the app-wide observable paths.

## Separate consumer and repeatable isolation check

`crates/ui/tests/consumer` is its own Cargo workspace and GPUI application.
It uses only public sofui controls, `init`, `mount`, theme application, retained
focus and text-change events. Its manifest names only sofui, GPUI and the
platform crate. Both it and sofui have dedicated lockfiles; shared transitive
package versions were checked against the root lockfile with no differences.

`scripts/rust/verify-sofui-isolation.sh` copies only sofui's declared manifest,
lockfile, README, source, gallery and consumer files to a temporary directory
outside the repository, then builds the gallery and consumer with `--offline
--locked`. It does not copy app/core source, a root manifest, target directories
or generated artifacts. `scripts/rust/verify --full` calls it, making
`make verify-full` cover this extraction boundary. The UI README documents
mounting, stable IDs/focus, text assignment/edit/undo, segmented controls,
holds, themes, declared dependencies/licenses, gallery and independent
consumer commands.

## Checks completed

- Scoped `rustfmt --edition 2021` on the edited Rust source files and
  `bash -n scripts/rust/verify scripts/rust/verify-sofui-isolation.sh` — passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true bash scripts/rust/verify-sofui-isolation.sh` — passed; copied-out gallery and consumer both built from their dedicated lockfiles.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo build --offline --locked -p sofui --example gallery` — passed. Binary: `/private/tmp/sofdevtool-terra-target/debug/examples/gallery`, SHA-256 `e47c106274c0d6202a25b98482f5c4219a20d2b7da0765782f2918c831ee8fe1`.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo build --offline --locked --manifest-path crates/ui/tests/consumer/Cargo.toml` — passed. Binary: `/private/tmp/sofdevtool-terra-target/debug/sofui-independent-consumer`, SHA-256 `44f50c005268496c600e7f529dfae365a528905011be83e30bd4934d992f7105`.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo check -p sofdevtool-app --all-targets` — passed after compatibility removal.
- Source audit found no `HistoryPanel`, `HistoryItem`, `Button::new`,
  `sofdevtool_ui`, `sofdevtool-ui`, `workbench_preview` or
  `--text-diff-proof` calls/paths in the maintained crates.

The coordinator owns the final full gate and native gallery/consumer runs.
These build results alone do not establish native visual or keyboard behavior.
No external repository or package publication was created.

## Native gallery density follow-up

The coordinator's first native gallery run showed `NumericStepper`'s
`Sample channel` label broken into five narrow fragments despite ample row
width. The component had assigned every label a fixed 24-pixel width. The
label now uses its natural no-wrap width without flex shrinking, and the
numeric value also keeps its width. The gallery's selectable-row sample labels
and previews are now neutral records rather than a Utility-specific hash and
snapshot example; this changes demonstration data only.

- `rustfmt --edition 2021 crates/ui/src/components/numeric_stepper.rs crates/ui/examples/gallery.rs` — passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo test -p sofui numeric_stepper_keyboard_path_clamps_and_disables_at_bounds` — passed (1 focused public interaction test).
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo clippy -p sofui --all-targets -- -D warnings` — passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo build -p sofui --example gallery` — passed. Updated gallery binary SHA-256: `c915bb184158dd5cda134068bb16878d9f4167c4068130248cfafb2ed0ed87d9`.
- The copied-out `bash scripts/rust/verify-sofui-isolation.sh` passed again
  with the revised component and sample data.

The coordinator owns the native recheck of the updated label; this worker did
not launch the gallery. The earlier native run alone does not prove the fix.

## Native independent-consumer verification

On macOS 26.2 arm64, the coordinator ran gallery and separate-consumer executables outside the checkout using minimal temporary `.app` wrappers containing only each binary and its Info.plist. No SofDevTool source, app/core crate, resources or data profile was supplied.

- Gallery binary `e47c106274c0d6202a25b98482f5c4219a20d2b7da0765782f2918c831ee8fe1`: selected Format B, incremented the sample channel, switched to Frappé then custom tokens, edited `caffè 👩🏽‍💻`, deleted the complete final emoji grapheme with Backspace, and restored it with Cmd-Z. AX showed the corresponding selection/theme/editor changes.
- Independent consumer binary `44f50c005268496c600e7f529dfae365a528905011be83e30bd4934d992f7105`: selected Comfortable, activated the counter button (`Pressed 1 times`), edited `Independent caffè 👩🏽‍💻`, deleted the complete emoji grapheme and restored it with Cmd-Z. These interactions used only the documented public interface.
- The first gallery screenshot exposed the narrow numeric label. After the small layout correction, binary `c915bb184158dd5cda134068bb16878d9f4167c4068130248cfafb2ed0ed87d9` visibly renders `Sample channel` on one readable line beside the value and buttons. Neutral Record A/B/C sample rows are also visible.

The combined gate preceding that final layout correction passed 338 Rust tests, 5 packaging tests, formatting, Clippy, Debug/Release builds, copied-out isolation, offline renderer reproduction and both app bundles. Final gate evidence is recorded separately under ticket22.
