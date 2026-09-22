# 21: Retire Swift and make maintained documentation describe the sole Rust app

**What to build:** The maintained tree contains one runnable Rust product with accurate setup/ownership guidance and no Swift/Xcode build requirement.

**Blocked by:** [20: Run and install distinct Debug/Release apps from the root workspace](20-root-workspace-packaging.md).

**Status:** ready-for-agent

- [ ] Before deletion, retain all needed independent fixtures, established icon resources and renderer source/provenance; source retirement leaves legacy installed apps and personal data untouched.
- [ ] Remove the Swift product/test targets, Xcode/Swift-only tooling, retired wrappers, executable spikes, obsolete plans/WIP and duplicate resources; Git history retains retired material.
- [ ] Complete the root README and agent guide with actual commands/source ownership, the component-only future sofui release rule, current domain contracts, profile/install behavior and evidence limits.
- [ ] Consolidate useful current contracts/evidence and repair retained links before old planning sources disappear. Root gates and renderer/fixture checks still pass with no hidden dependency on retired files.

## Testing

Fresh-checkout root commands, dependency/resource audit, retained contract fixtures and complete maintained-document link checks.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 01, 03, 04, 05, 06, 08, 55, 57.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
