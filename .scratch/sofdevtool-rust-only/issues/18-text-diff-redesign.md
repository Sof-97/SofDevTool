# 18: Redesign Text Diff and preserve native WebView interactions

**What to build:** The approved Text Diff workspace supports editing, split/unified comparison and exact History restore with correct clipping, focus, scrolling and Copy.

**Blocked by:** [04: Propagate History changes through generators, Regex and Text Diff](04-history-generator-comparison-workspaces.md); [12: Apply the approved design to navigation, Launcher and Settings](12-workbench-launcher-settings-redesign.md); [17: Make Text Diff assets independently maintainable and locally bundled](17-text-diff-asset-pipeline.md).

**Status:** ready-for-agent

- [ ] Adopt approved editors, controls, diagnostics and generic History presentation; preserve the application-owned replaceable renderer boundary.
- [ ] Exercise Unicode and disclosed complex-emoji fallback, exact old/new text restore, readiness, stale result rejection and visible recovery.
- [ ] Verify resize/clipping, selection, scrolling, native Copy and focus handoff between GPUI and the embedded WebView in a running application.
- [ ] Remove this batch's old component calls while retaining local-only resources and the independently maintainable asset pipeline.

## Testing

Real packaged/native renderer interactions, owning snapshot/session contracts and isolated History.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 33, 49, 50, 51, 53, 54, 58, 61, 66, 69.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
