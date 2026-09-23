# 20: Run and install distinct Debug/Release apps from the root workspace

**What to build:** Root commands build and launch the correctly identified Debug app, package Release and install the intended Release artifact to an overridable destination.

**Blocked by:** [01: Use sofui's owned component interfaces in JSON and the gallery](01-sofui-json-gallery.md); [17: Make Text Diff assets independently maintainable and locally bundled](17-text-diff-asset-pipeline.md).

**Status:** claimed

- [ ] Relocate the three-crate Cargo workspace to the root with pinned toolchain/dependencies intact and make help/run/gallery/format/verify/verify-full/release/install working from a fresh checkout.
- [ ] Use SofDevTool 0.2.0 and the exact agreed Release/Debug names, bundle IDs, data namespaces and established normal/orange-DEV artwork; include deliberate build identifier, immutable revision and channel from authoritative metadata.
- [ ] Keep profile bundles and preferences/History/Random String controls separate, preserve test-data-root override and legacy data, and honor a nondefault Cargo target directory.
- [ ] Verify packaging/install failures propagate and replacement stays within the intended bundle. Document the default home Applications destination and override; exercise a temporary destination now, reserving the actual default install acceptance for 22.

## Testing

Root command execution, bundle metadata/resources and isolated profile persistence; install to a temporary destination without touching personal data.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 02, 09, 10, 11, 12, 13, 14, 16, 17, 18, 19, 48, 70.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

## Implementation checkpoint — 2026-09-23

Root workspace, separate Debug/Release identity, authoritative bundle metadata, retained artwork, root Makefile and scoped installer are implemented. `make verify` and `make verify-full` pass; temporary-destination install and packaging failure checks pass. See [implementation evidence](../evidence/ticket-20.md). Actual default installation and final native Release acceptance belong to ticket 22.
