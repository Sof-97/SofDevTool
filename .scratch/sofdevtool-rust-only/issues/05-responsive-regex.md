# 05: Keep Regex responsive and reject obsolete evaluations

**What to build:** A demanding Regex request runs while the app stays interactive, and only the newest complete valid result can appear or enter History.

**Blocked by:** None (can start immediately).

**Status:** resolved

- [x] Move actual compilation, matching, capture collection and replacement off the UI thread; bound running work and pending requests.
- [x] New edits, clear and restore invalidate old work. Reject obsolete results and snapshots, and check cancellation between controllable stages.
- [x] Preserve all specified pattern/program/text/replacement/match/capture/output budgets, dialect diagnostics and replacement guidance; do not present partial success.
- [x] Remove the blanket linear-time claim. Demonstrate both controlled stale completion and a demanding real-engine case without promising interruption inside an engine call.

## Testing

Regex workspace scheduling and observable UI/session progress, with real-engine limits and controlled completion order.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 34, 35, 36, 68.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

**Execution note (2026-09-23):** The first pi/Kimi attempt stopped during source
inspection because OpenCode Go returned HTTP 429 `GoUsageLimitError`. No product
files changed. The preserved session can be resumed when that provider is
available; acceptance remains open. See [attempt evidence](../evidence/ticket-05.md).

**Implementation:** GPT-6 Sol candidate integrated with focused checks on 2026-09-23; see [evidence](../evidence/ticket-05.md). Native acceptance and final Astra review remain open; status stays claimed.

## Acceptance closeout — 2026-09-23

The four clauses are supported by [ticket-specific evidence](../evidence/ticket-05.md), the [final independent source review](../evidence/final-review.md) and the corrected-source [full gate and installed observations](../evidence/ticket-22.md). Earlier implementation notes describe their then-current state; this closeout records the coordinator-approved integration decision. Ticket 22 retains separate final Release and local-delivery acceptance.
