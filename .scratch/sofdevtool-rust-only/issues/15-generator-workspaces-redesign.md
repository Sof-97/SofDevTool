# 15: Redesign Identifier Generator, Random String and Sample Data

**What to build:** All three generators use the approved controls and lists with safe restoration, saved Random String configuration and intact generation behavior.

**Blocked by:** [04: Propagate History changes through generators, Regex and Text Diff](04-history-generator-comparison-workspaces.md); [07: Protect generated results when restoring History](07-generator-restore-confirmation.md); [08: Remember Random String controls independently of History](08-random-string-preferences.md); [12: Apply the approved design to navigation, Launcher and Settings](12-workbench-launcher-settings-redesign.md).

**Status:** resolved

- [x] Retain every UUID/ULID/KSUID mode and bound, Random String cryptographic/alphabet rules, and Sample Data schema/row/field/JSON/CSV behavior.
- [x] Use reusable selection, stepping, lists and confirmation mechanics while keeping formats, schema logic, entropy and generation in app/core ownership.
- [x] Exercise per-result Copy/Copy All, repeated explicit generations, output-aware restore confirmation, edited schemas and preferences with History disabled.
- [x] Migrate these History presentations, remove compatibility calls and preserve exact captured output without rerunning generators.

## Testing

Complete generator workspace flows with deterministic random/clock adapters, preferences and temporary History.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 33, 45, 46, 47, 48, 49, 51, 58, 61, 63, 66.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

## Acceptance closeout — 2026-09-23

The four clauses are supported by [ticket-specific evidence](../evidence/ticket-15.md), the [final independent source review](../evidence/final-review.md) and the corrected-source [full gate and installed observations](../evidence/ticket-22.md). Earlier implementation notes describe their then-current state; this closeout records the coordinator-approved integration decision. Ticket 22 retains separate final Release and local-delivery acceptance.
