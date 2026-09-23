# 22: Verify the installed Release and deliver a clean local branch

**What to build:** The owner receives a tested Release installation and a clean, committed local branch whose evidence covers the complete agreed product.

**Blocked by:** [19: Finish sofui extraction readiness and remove compatibility scaffolding](19-sofui-independent-consumer.md); [21: Retire Swift and make maintained documentation describe the sole Rust app](21-swift-retirement-documentation.md).

**Status:** resolved

- [x] Run the complete appropriate regression/component/asset/build gates and fifteen-Utility acceptance on the final integrated candidate, retaining accurate revision/host/toolchain evidence. The latest `make verify-full` gate passed 339 Rust and five Python tests plus assets, independent component consumption, Debug/Release bundles and identity. All fifteen Utilities then passed representative native smoke flows on the same installed r4 Release; the preceding candidate's more detailed variant matrix remains distinct in [ticket 22 evidence](../evidence/ticket-22.md).
- [x] Actually install to the default home Applications location and launch outside the checkout with isolated data; verify identity/version/revision/channel, resources, a recorded operation and persistence across relaunch. The latest r4 Release at `/Users/gerardo/Applications/SofDevTool.app` displayed 0.2.0 build 1, release, clean revision `b4e17030`, and restored a persisted synthetic JSON query/result from the isolated profile. Prior corrected Text Diff native rendering and persistence are recorded separately.
- [x] Record actual native Launcher/Spaces/Dock, editing, themes and WebView checks separately from builds. Coordinator-observed Launcher, editing, themes and WebView checks are distinct from the owner's reported passes of the physical global chord from another app, another Space/full-screen context, and literal Dock return after Cmd-W. macOS 14/15 runtime, cross-display/DPI transitions and IME composition remain unverified or excepted as specified; personal History was not inspected.
- [x] Review the complete diff for standards/spec compliance, resolve findings and leave a clean merge-ready local branch using GitButler. Independent reviews and all three source findings are resolved. Before this documentation-only closeout, branch `feat/rust-gpui-migration` at `89fd007` had the expected base ancestry and clean worktree/index after the coordinator's scoped index reconciliation. The coordinator owns committing these final status documents and rechecking the resulting branch; no future commit hash is asserted here. No remote push, PR, merge, library publication or unrelated cleanup is included.

## Testing

Final integrated automated gates and the actually installed native Release, followed by fixed-revision review and local branch verification.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 20, 49, 54, 68, 69, 70, 71.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
