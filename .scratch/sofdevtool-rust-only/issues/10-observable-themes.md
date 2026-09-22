# 10: Apply observable themes across all open surfaces and editors

**What to build:** Changing Graphite or Catppuccin Frappé updates Workbench, Launcher, Settings and wrapped editors consistently while preserving work.

**Blocked by:** [01: Use sofui's owned component interfaces in JSON and the gallery](01-sofui-json-gallery.md).

**Status:** ready-for-agent

- [ ] Provide observable application-wide semantic tokens, both existing presets and custom tokens; Graphite remains the fresh default.
- [ ] Keep persisted theme choice in the application. Consumers do not need manual per-window redraw recipes or direct dependency-internal theme types.
- [ ] Verify changes across multiple open windows without recreating sessions, losing focus context, changing Recents or generating History.
- [ ] Demonstrate public customization in the gallery and preserve useful focus/diagnostic contrast; independent per-window themes remain outside scope.

## Testing

Application multi-window theme changes and public sofui customization; observable session identity/content and editor appearance.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 30, 31, 62, 66.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
