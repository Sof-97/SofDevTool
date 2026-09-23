# 11: Present one native Workbench preview for owner approval

**What to build:** The owner can run and review one compact native GPUI design using actual sofui controls before it is applied across the product.

**Blocked by:** [06: Synchronize color controls, output and retained operations](06-synchronized-color.md); [09: Use generic lists, holds and confirmations for JSON History](09-generic-history-interactions.md); [10: Apply observable themes across all open surfaces and editors](10-observable-themes.md).

**Status:** resolved

- [x] Show realistic catalog density, central editor/result, trailing History, representative selection/numeric/color/confirmation controls and both theme presets.
- [x] Use synthetic in-memory data and public controls, with enough resizing, focused, invalid and disabled states to assess density and behavior.
- [x] Record the running native preview and visual evidence; incorporate requested feedback within the agreed direction.
- [x] Completion requires explicit owner approval recorded with the reviewed candidate. A working preview alone does not unblock 12.

## Testing

The actual native GPUI preview and human design review; screenshots support appearance but do not prove interactions.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 21, 22, 32, 63, 69.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

## Preview candidate

GPT-6 Sol supplied the compiling native example and its
[evidence](../evidence/ticket-11.md). No owner approval has yet been recorded;
12 remains blocked pending review of the runnable candidate.

## Acceptance closeout — 2026-09-23

The four clauses are supported by [ticket-specific evidence](../evidence/ticket-11.md), the [final independent source review](../evidence/final-review.md) and the corrected-source [full gate and installed observations](../evidence/ticket-22.md). Earlier implementation notes describe their then-current state; this closeout records the coordinator-approved integration decision. Ticket 22 retains separate final Release and local-delivery acceptance.
