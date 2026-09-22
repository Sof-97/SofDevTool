# 16: Redesign Regex and Color Conversion without regressing corrections

**What to build:** The approved comparison/selection interfaces keep Regex responsive and all color representations synchronized through Copy and History.

**Blocked by:** [03: Propagate History changes through conversion and inspection workspaces](03-history-conversion-workspaces.md); [04: Propagate History changes through generators, Regex and Text Diff](04-history-generator-comparison-workspaces.md); [05: Keep Regex responsive and reject obsolete evaluations](05-responsive-regex.md); [12: Apply the approved design to navigation, Launcher and Settings](12-workbench-launcher-settings-redesign.md).

**Status:** ready-for-agent

- [ ] Adopt the approved controls and generic History presentation in both workspaces, removing this batch's old API calls.
- [ ] Re-exercise demanding Regex evaluation, bounded scheduling, stale completion, clear/restore and all resource refusals through the redesigned UI.
- [ ] Re-exercise consecutive color adjustments around settlement, invalid text, channel controls, swatch/output/Copy agreement and single-operation recording.
- [ ] Preserve the specified Regex semantics and bounded sRGB representations; add no unsupported dialect/color-space features.

## Testing

Real workspace controls plus controlled asynchronous completion and the regression checks from 05 and 06.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 33, 34, 35, 36, 43, 44, 49, 50, 51, 58, 61, 63, 66.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
