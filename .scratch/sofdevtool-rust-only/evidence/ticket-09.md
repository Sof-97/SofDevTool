# Ticket 09: generic lists, holds and confirmations

Implementation candidate, 2026-09-23. Coordinator owns integration and native
launch. No application/core manifest or History repository changes were needed.

## Behavior

- `SelectableList` accepts caller-supplied stable row IDs, title, summary,
  empty message, preview, status, disabled state and selection action. A
  required retained `SelectableListFocus` keeps row focus across redraws;
  pointer, Enter and Space select the same row. JSON uses it directly, supplies
  the History title/quota/unavailable wording, and retains its own selected ID
  and restore policy. `HistoryPanel` remains a temporary compatibility adapter
  for the other workspaces and delegates presentation to the generic list.
- `HoldButton` requires a retained `HoldController` and caller duration. sofui
  owns the controlled timer, progress, one-shot completion, early release,
  pointer-exit and Escape cancellation. Enter, Space and accessibility Click
  request ordinary confirmation through a separate callback; none directly
  performs the destructive action. Settings supplies one second for Clear
  Utility and two seconds for Clear All, owns the History mutation and result
  notice, and retains controllers/focus per known and unknown Utility.
- `ConfirmationBar` provides generic inline message/buttons/focus and
  confirm/cancel actions. JSON uses it for restore replacement and Settings for
  keyboard History deletion. The gallery demonstrates selected, unavailable,
  disabled and failed generic rows, an actual empty list, hold and confirmation.
  No snapshot format, retention policy or product identity moved into the
  generic controls.

## Verification

Commands ran from `rust/` with `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target`
and `CARGO_NET_OFFLINE=true`.

| Command | Result |
| --- | --- |
| `cargo test -p sofui` | Passed: 5 editor, 3 general public interaction, 2 new hold/list/confirmation pointer and keyboard, 1 theme tests (11 total). Controlled clock covers release, exit, Escape, progress, one completion, confirmation request/cancel/accept and list focus/selection. |
| `cargo test -p sofdevtool-app settings::tests::` | Passed: 9 tests, including a new real Settings keyboard confirm/cancel and two-second pointer hold that deletes only isolated synthetic History. Existing partial-clear, failure/recovery and live-reconciliation tests pass. |
| `cargo test -p sofdevtool-app selectable_history_and_confirmation_reconcile_after_settings_deletion` | Passed: real JSON list and confirmation keyboard controls; restore cancellation preserves current input, confirmation restores exactly without another entry, and deletion invalidates selected/pending entries without clearing active input. Uses an isolated synthetic History root. |
| `cargo clippy -p sofui -p sofdevtool-app --all-targets -- -D warnings` | Passed. |
| `rustfmt --edition 2021 --check` on owned Rust files | Passed. |
| `cargo test -p sofdevtool-app` | Current shared source: 85/86 passed. One unrelated in-progress Text Diff renderer protocol test failed at `renderer.rs:395` because the actively edited checked-in Pierre bundle did not contain each bridge needle once. Coordinator routed this to ticket 17 owner and will run a frozen combined gate. Ticket 09 tests passed in this run. |

The GPUI tests use synthetic windows and deterministic clock time. They do not
establish native Workbench/Settings appearance or macOS 14/15 runtime behavior;
the coordinator owns native acceptance.

## Coordinator integration gate

A frozen `e1aa178` plus the exact09 candidate passed the full offline default
gate in `/private/tmp/sofdevtool-wave5-fwoioc02/rust`: formatting, all-target
Clippy with warnings denied, 310 tests (86 app,190 core,23 JSON,11 sofui),
and Debug app/gallery builds. Active17 asset edits were excluded from this
snapshot. Log: `.artifacts/parallel-rust-only/wave5/verify.log`.

The preceding frozen04 run exposed one intermittent initial History write
failure in a Settings test. Its temporary-root helper now includes a
process-local atomic sequence in addition to PID/time, preventing colliding
parallel test directories. The full gate passed with this isolation fix.

After the full gate, ten consecutive Settings test runs passed on the same
frozen candidate (90 successful tests). Log:
`.artifacts/parallel-rust-only/wave5/settings-repeat.log`.
