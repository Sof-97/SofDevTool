# 12: Apply the approved design to navigation, Launcher and Settings

**What to build:** The redesigned shell supports the familiar catalog/navigation and native Launcher/Settings workflows while retaining every opened Utility Workspace Session.

**Blocked by:** [11: Present one native Workbench preview for owner approval](11-native-preview-approval.md).

**Status:** resolved

- [x] Preserve Library/Recent/Favorites, grouping, search, keyboard selection, separate Settings and trailing collapsible History.
- [x] Use the approved component treatment for shell controls; old workspace bodies remain usable until their individual rollout tickets land.
- [x] Exercise configurable shortcut failure, nonactivating dismissal, current-Space/full-screen behavior, opening a Utility, Dock reopen, closing the last window and explicit Quit.
- [x] Verify switching and theme changes preserve input/results, focus where applicable and native editor/WebView handoff; report runtime limits honestly.

## Testing

Actual application navigation/native flows plus existing registry/session tests and isolated settings.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 23, 24, 25, 26, 28, 29, 32, 69.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

## Acceptance closeout — 2026-09-23

All four clauses are supported by [ticket-specific evidence](../evidence/ticket-12.md), the [final source review](../evidence/final-review.md) and the [installed Release observations](../evidence/ticket-22.md). The coordinator observed shell, theme, lifecycle and WebView behavior with an isolated profile. After being asked to test the physical global shortcut from another app, another Space/full-screen context and literal Dock return after Cmd-W, the owner replied “Si sembra funzionare tutto.” These three are owner-reported passes, not CUA-observed events; earlier synthetic-key and application-open limitations remain accurately recorded as historical observations. macOS 14/15 runtime and cross-display/DPI transitions were not tested.
