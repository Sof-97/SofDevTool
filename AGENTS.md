# SofDevTool agent guide

Read `CONTEXT.md` and `.scratch/sofdevtool/implementation-handoff.md` before changing product behavior.

## Source map

- `SofDevTool/Application`: lifecycle and dependency composition.
- `SofDevTool/Shell`: Workbench, Launcher, Settings, and catalog state.
- `SofDevTool/Registry`: source-defined Utility catalog and search.
- `SofDevTool/History`: snapshots, recording policy, and local repository.
- `SofDevTool/Platform`: explicit Clipboard, shortcut, window, and activation adapters.
- `SofDevTool/Utilities`: independent Utility modules and their workspace sessions.
- `SofDevToolTests` and `SofDevToolUITests`: contract and focused shell verification.

## Commands

- Apply formatting: `scripts/format`
- Default gate: `scripts/verify`
- Release and UI gate: `scripts/verify --full`

## Rules

- Preserve strong Utility types; type-erase only workspace construction in the registry.
- Do not introduce a universal execute-input interface.
- Clipboard reads/writes are explicit. History contains completed valid Utility Operations only.
- Never log, sync, index, or export History payloads.
- Put domain tests with the owning Utility; use UI tests only for shell and interaction-heavy flows.
- Before finishing, run the relevant gate and update evidence honestly. macOS 14/15 deployment builds are not runtime checks.
