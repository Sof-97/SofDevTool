# 07: Protect generated results when restoring History

**What to build:** Identifier Generator and Sample Data ask before replacing a different nonempty workspace, even when generation settings match.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Include generated output and meaningful edited controls/schema in restore equivalence; an edited invalid Sample Data schema is not an empty workspace.
- [ ] Test two different generated batches with identical settings: restore prompts, cancellation preserves current state and confirmation restores exact captured output.
- [ ] Equivalent and empty sessions avoid unnecessary prompts; restoration never calls randomness or the clock and never records a new operation.
- [ ] Ensure obsolete pending work cannot overwrite a restored session; retain existing generator limits and format semantics.

## Testing

Generator workspace restore decisions and application confirmation, with deterministic random/clock spies and captured snapshots.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 45, 46, 68.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
