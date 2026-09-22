# 07: Navigate retained Utility sessions with themed components

Type: task
Status: claimed
Blocked by: 06

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Switch between JSON and Base64 through searchable Library, Recent and Favorites scopes, preserving sessions and applying a coherent dark theme across Workbench, Launcher and Settings.

## Acceptance criteria

- [ ] Deliver compact toolbar, grouped catalog, flexible workspace and collapsible History, with useful minimum-window resizing behavior.
- [ ] Persist Rust favorites, recents, theme and meaningful settings, with safe defaults for missing/invalid preferences; never persist session inputs outside enabled History.
- [ ] Preserve sessions and focus through catalog, scope, theme and Settings changes; test two real Utilities rather than placeholders.
- [ ] Keep Graphite default and Catppuccin Frappé available, with semantic tokens, notices and readable normal/selected/disabled/error/focus states.
- [ ] Extend the independent gallery with the actual list/select/panel/dialog/feedback components used here, without application dependencies.
- [ ] Verify UI flows plus session/persistence contracts; no full VoiceOver or pixel-perfect Swift parity gate is required.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Implemented; automated gate passes and evidence is recorded in [`rust/docs/evidence/ticket-07.md`](../../../rust/docs/evidence/ticket-07.md). Native host verification (pointer hold, keyboard confirmation, theme/scope/session preservation) is pending the final ticket-22 pass; macOS 14/15 runtime remain unverified.
