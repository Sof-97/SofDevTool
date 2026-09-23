# SofDevTool agent guide

Read `CONTEXT.md` before changing product behavior. Inspect the owning module and its tests for current Utility limits, History/privacy policy and native interaction contracts. Use the root `README.md` for maintained commands.

## Rust component ownership

`crates/ui` is **sofui 0.1.0**, an independently versioned GPUI component library intended for a later separate release. Keep it limited to reusable UI components and component interaction logic, including its text-editing engine. The application and core own Utility execution, domain validation, persistence, History policy and lifecycle. Initialize and mount sofui in each consumer; start GPUI and configure process hooks in the consumer's entry point. Verify the public component interface from an independent consumer when changing it.

## Source map

- `crates/app`: lifecycle, Workbench, Launcher, Settings, Registry, History coordination, Clipboard, native adapters and Utility workspaces.
- `crates/core`: domain evaluation, validation, snapshots and independent Utility contract tests.
- `crates/ui`: reusable sofui controls, themes and component interaction only.
- `platform/macos`: icons and application notices; `scripts/rust`: root packaging, install and verification scripts.
- `crates/app/src/text_diff/assets-source`: editable embedded renderer, lockfile, verification and provenance.

## Commands

- `make help` lists supported root workflows. `make run` opens Debug; `make gallery` opens sofui's component gallery.
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
