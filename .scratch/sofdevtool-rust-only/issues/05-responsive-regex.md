# 05: Keep Regex responsive and reject obsolete evaluations

**What to build:** A demanding Regex request runs while the app stays interactive, and only the newest complete valid result can appear or enter History.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Move actual compilation, matching, capture collection and replacement off the UI thread; bound running work and pending requests.
- [ ] New edits, clear and restore invalidate old work. Reject obsolete results and snapshots, and check cancellation between controllable stages.
- [ ] Preserve all specified pattern/program/text/replacement/match/capture/output budgets, dialect diagnostics and replacement guidance; do not present partial success.
- [ ] Remove the blanket linear-time claim. Demonstrate both controlled stale completion and a demanding real-engine case without promising interruption inside an engine call.

## Testing

Regex workspace scheduling and observable UI/session progress, with real-engine limits and controlled completion order.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 34, 35, 36, 68.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
