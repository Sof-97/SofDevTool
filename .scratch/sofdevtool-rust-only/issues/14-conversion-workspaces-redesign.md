# 14: Redesign Hashes, Timestamps, Case and Whitespace Conversion

**What to build:** Four conversion Utilities use the approved interface while preserving their distinct actions, policies and recorded results.

**Blocked by:** [03: Propagate History changes through conversion and inspection workspaces](03-history-conversion-workspaces.md); [12: Apply the approved design to navigation, Launcher and Settings](12-workbench-launcher-settings-redesign.md).

**Status:** claimed

- [ ] Migrate all four workspaces and their History presentation; use demonstrated selection/numeric controls rather than Utility logic in sofui.
- [ ] Preserve explicit hashing including empty bytes/legacy labels, timestamp inference/DST diagnostics, all case styles and whitespace actions/ranges.
- [ ] Verify neutral/invalid states, Copy, switching and exact restore; retain independent expected values and Unicode regressions.
- [ ] Remove this batch's compatibility calls and keep the application green before other batches finish.

## Testing

Owning Utility contracts and public workspace actions, with isolated History and controlled time where needed.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 33, 49, 50, 51, 58, 59, 61, 63, 66.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
