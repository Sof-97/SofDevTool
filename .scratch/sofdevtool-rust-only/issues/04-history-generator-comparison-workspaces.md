# 04: Propagate History changes through generators, Regex and Text Diff

**What to build:** The remaining five Utilities respond consistently to History changes, completing immediate invalidation across the entire catalog.

**Blocked by:** [02: Coordinate JSON and Base64 History with enforced recording recovery](02-history-coordination-recovery.md).

**Status:** claimed

- [ ] Migrate Identifier Generator, Random String, Sample Data, Regex and Text Diff to the shared coordinator without erasing their Utility-owned snapshot types.
- [ ] Deleted entries cannot remain selected, previewable or restorable; current generated results, comparison inputs and in-flight session revisions remain intact.
- [ ] Preview and restore reproduce captured state without fresh random/clock execution or a new History entry; deliberate repeated operations still record separately.
- [ ] Keep the conversion batch independently green. Record completion of this consumer batch without claiming the whole migration unless 03 has also completed.

## Testing

Application sessions, controlled generation/completion order and temporary History persistence.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 37, 38, 41, 42, 46, 51.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

## Implementation handoff

GPT-6 Sol completed all five consumers and actual-workspace regressions;
85 app tests, owned formatting and strict all-target Clippy passed. See
[evidence](../evidence/ticket-04.md). Final native Text Diff acceptance and
the comprehensive Astra review remain outstanding.
