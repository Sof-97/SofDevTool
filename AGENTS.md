# SofDevTool agent guide

Read `CONTEXT.md` before changing product behavior. Inspect the owning module and its tests for current Utility limits, History/privacy policy and native interaction contracts. Use the root `README.md` for maintained commands.

## Rust component ownership

The application consumes [GPUI Kit](https://docs.rs/crate/gpui-kit/0.6.6) directly: controls, appearance and text editing come from the kit, initialized and mounted by the consumer. `crates/app` owns Utility execution, domain validation, persistence, History policy and lifecycle; `crates/core` owns the Utility domain engines. Keep app-specific Utility compositions and the specialised Text Diff renderer as explicit boundaries, not as a replacement component library. There is no `sofui` crate, gallery or independent component release.

The owner-approved editing exception is `crate::ui::multiline_editor`: retain its narrow Backspace/Delete adapter for extended graphemes, including partial selections. Rendering, selection, IME, undo/redo and scrolling remain kit-owned. Before replacing this adapter, verify equivalent upstream behavior; see `.scratch/gpui-kit-migration/issues/10-native-acceptance-record.md`, item 1.

## Source map

- `crates/app`: lifecycle, Workbench, Launcher, Settings, Registry, History coordination, Clipboard, native adapters and Utility workspaces.
- `crates/core`: domain evaluation, validation, snapshots and independent Utility contract tests.
- `platform/macos`: icons and application notices; `scripts/rust`: root packaging, install and verification scripts.
- `crates/app/src/text_diff/assets-source`: editable embedded renderer, lockfile, verification and provenance.

## Commands

- `make help` lists supported root workflows. `make run` opens Debug.
- `make format` applies Rust formatting; `make verify` runs the default gate.
- `make verify-full` adds Release and renderer/bundle verification; `make release` packages Release and `make install` installs it, with `INSTALL_DESTINATION` for a temporary destination.

## Rules

- Preserve strong Utility types; type-erase only workspace construction in the registry.
- Do not introduce a universal execute-input interface.
- Clipboard reads/writes are explicit. History contains completed valid Utility Operations only.
- Never log, sync, index, or export History payloads.
- Put domain tests with the owning Utility; use UI tests only for shell and interaction-heavy flows.
- Before finishing, run the relevant root gate and report validation honestly. A build, automated test or temporary install is not native runtime evidence on macOS 14/15.

## Agent skills

### Issue tracker

Issues and specs live in local Markdown files under `.scratch/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the five default triage roles as local status strings. See `docs/agents/triage-labels.md`.

### Domain docs

Use the single-context layout with root `CONTEXT.md` and `docs/adr/`. See `docs/agents/domain.md`.
