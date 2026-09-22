# 08: Remember Random String controls independently of History

**What to build:** Random String reopens with the latest configuration even when History is disabled and no generation has occurred.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Persist all specified controls, including custom alphabet and ambiguous-character exclusion, with validated/versioned loading and the agreed defaults/ranges.
- [ ] Reconstruct the workspace under an isolated profile and recover controls without generating output or recording History.
- [ ] Keep generated values and operation bookkeeping out of ordinary preferences; distinguish missing, malformed and failed-save states honestly.
- [ ] Use the existing profile/data-root seam so final Debug/Release isolation can be exercised by ticket 20 without adding migration of legacy user data.

## Testing

Random String workspace/preferences reconstruction using temporary data roots and deterministic generation adapters.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 47, 48, 68.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
