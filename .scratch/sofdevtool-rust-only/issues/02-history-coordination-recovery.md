# 02: Coordinate JSON and Base64 History with enforced recording recovery

**What to build:** Settings deletion immediately updates open JSON and Base64 History views, and a write failure really pauses recording until a coherent recovery succeeds.

**Blocked by:** None (can start immediately).

**Status:** claimed

- [ ] Expand application-owned coordination and adapt JSON, Base64 and Settings first; retain an adapter for the other workspaces while they migrate.
- [ ] Clear one/all removes affected retained entries, selection and pending restore in both open workspaces, including a hidden one, while preserving current workspace content.
- [ ] Enforce per-Utility pause at the shared recording boundary used by all callers. Failed writes preserve the previous file/result; ordinary operations stay paused; retry/relaunch follows the specified corruption and duplicate rules.
- [ ] Verify global/per-Utility policy, newest-25 retention, unavailable entries, atomic failure, partial deletion and unknown Utility files. This ticket does not claim all-workspace invalidation until 03 and 04 complete.

## Testing

Application sessions and Settings against temporary History storage, with deterministic failure and recovery.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 39, 40, 41, 42, 51.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

## Implementation handoff

GPT-6 Sol completed the candidate on 2026-09-23. Shared coordination is in use
by JSON, Base64 and Settings; other workspaces migrate in03/04. Coordinator
integration requested a Clippy fix and legacy-filename deletion coverage; both
are included. 69 app tests, all-target Clippy and owned formatting passed.
[Evidence](../evidence/ticket-02.md). Native acceptance and final Astra review
remain pending, so the ticket stays claimed.
