# 09: Use generic lists, holds and confirmations for JSON History

**What to build:** JSON History and Settings deletion use reusable list/confirmation controls whose hold progress and cancellation work without application-managed timers.

**Blocked by:** [01: Use sofui's owned component interfaces in JSON and the gallery](01-sofui-json-gallery.md); [02: Coordinate JSON and Base64 History with enforced recording recovery](02-history-coordination-recovery.md).

**Status:** claimed

- [ ] sofui owns reusable timing/progress, selection, focus and confirmation mechanics; application code supplies labels, quota, duration, destructive action and recording policy.
- [ ] Preserve one-second Clear Utility and two-second Clear All holds, cancellation on release/exit/Escape, and ordinary confirmation for keyboard activation.
- [ ] Demonstrate empty, unavailable, selected, disabled and failure states using generic gallery data and real JSON/Settings actions.
- [ ] Keep temporary adapters for unmigrated History presentations; final product-vocabulary and old-API removal belongs to 19 after the rollout batches.

## Testing

Public pointer/keyboard component behavior with a controlled clock, plus JSON/Settings deletion against isolated History.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 27, 60, 61, 63, 66.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
