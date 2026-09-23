# 13: Redesign JSON, YAML/JSON, Base64, URL Encoding and JWT

**What to build:** Five text/inspection Utilities use the approved sofui controls from input through diagnostics, Copy and exact History restore.

**Blocked by:** [03: Propagate History changes through conversion and inspection workspaces](03-history-conversion-workspaces.md); [12: Apply the approved design to navigation, Launcher and Settings](12-workbench-launcher-settings-redesign.md).

**Status:** resolved

- [x] Migrate the five complete workspace flows, including their generic History presentation, to the new interfaces; remove this batch's compatibility calls.
- [x] Preserve all modes, independent vectors and fidelity/decoding/query semantics in the canonical specification; JWT remains inspection-only with History off by default.
- [x] Exercise invalid/neutral input, explicit Clipboard actions, Unicode editing/undo, session switching and exact restore without re-execution.
- [x] Demonstrate each Utility and keep this batch green with the other workspace groups still using their old interfaces.

## Testing

Existing Utility core contracts plus actual workspace/component wiring and representative application History flows.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 33, 49, 50, 51, 52, 58, 59, 61, 63, 66.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

## Acceptance closeout — 2026-09-23

The four clauses are supported by [ticket-specific evidence](../evidence/ticket-13.md), the [final independent source review](../evidence/final-review.md) and the corrected-source [full gate and installed observations](../evidence/ticket-22.md). Earlier implementation notes describe their then-current state; this closeout records the coordinator-approved integration decision. Ticket 22 retains separate final Release and local-delivery acceptance.
