# 22: Verify the installed Release and deliver a clean local branch

**What to build:** The owner receives a tested Release installation and a clean, committed local branch whose evidence covers the complete agreed product.

**Blocked by:** [19: Finish sofui extraction readiness and remove compatibility scaffolding](19-sofui-independent-consumer.md); [21: Retire Swift and make maintained documentation describe the sole Rust app](21-swift-retirement-documentation.md).

**Status:** claimed

- [x] Run the complete appropriate regression/component/asset/build gates and fifteen-Utility acceptance on the final integrated candidate, retaining accurate revision/host/toolchain evidence. The latest `make verify-full` gate passed 339 Rust and five Python tests plus assets, independent component consumption, Debug/Release bundles and identity. All fifteen Utilities then passed representative native smoke flows on the same installed r4 Release; the preceding candidate's more detailed variant matrix remains distinct in [ticket 22 evidence](../evidence/ticket-22.md).
- [x] Actually install to the default home Applications location and launch outside the checkout with isolated data; verify identity/version/revision/channel, resources, a recorded operation and persistence across relaunch. The latest r4 Release at `/Users/gerardo/Applications/SofDevTool.app` displayed 0.2.0 build 1, release, clean revision `b4e17030`, and restored a persisted synthetic JSON query/result from the isolated profile. Prior corrected Text Diff native rendering and persistence are recorded separately.
- [ ] Record actual native Launcher/Spaces/Dock, editing, themes and WebView checks separately from builds. The Launcher button, Escape, Cmd-W process survival, editing, themes and WebView have native observations; the physical global chord, another Space/fullscreen, and literal Dock return still await the owner's manual check. Keep untested OS/DPI limits and the accepted IME exception explicit; never inspect personal History.
- [ ] Review the complete diff for standards/spec compliance, resolve findings and leave a clean merge-ready local branch using GitButler. The independent reviews and all three source findings are resolved; local base ancestry and branch/synthetic-tree equivalence passed preflight, while the coordinator must repeat final cleanliness/integration checks after documentation changes. No remote push, PR, merge, library publication or unrelated cleanup is included.

## Testing

Final integrated automated gates and the actually installed native Release, followed by fixed-revision review and local branch verification.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 20, 49, 54, 68, 69, 70, 71.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
