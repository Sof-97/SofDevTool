# Final independent source review — findings resolved

The coordinator requested independent Astra Standards and Specification reviews of the integrated Rust-only change from fixed base `4db64951d5d00b814eeeb03b8941b1ebc04f1b19` through reviewed source `0f077e0`. Each axis retains its own finding count and resolution below.

## Standards

**0 findings.** The independent Standards review reported no actionable standards issue in the reviewed range. No Standards fix or severity transfer from the other axis is claimed.

## Specification

**2 P2 findings; both resolved on recheck.** A fresh Text Diff workspace still started with hard-coded demonstration input, and editing could record an intermediate valid comparison on each keystroke rather than only one settled Utility Operation. These were source findings, not native-test conclusions.

GitButler commit `df95bd39b281e4d7e44375ef8c7543417b01490e` removed the demonstration editor values, left initial renderer readiness neutral, and added a cancelable 200 ms settlement boundary for current-revision History recording. Edits and mode changes supersede a pending settlement; restore and the debug-failure path cancel it. Headless GPUI tests now cover empty startup, rapid Unicode edits, one latest settled record after readiness, neutral clearing, and no late recording after restore. See [ticket 18](ticket-18.md) for the scoped correction and tests. Astra rechecked the corrected source and reported both Specification P2 findings resolved with no new findings.

After that two-finding review, installed Launcher diagnosis found a separate Carbon event-kind defect: the registrar subscribed to Released (`6`) under a Pressed name. Sol's `33d68b3` correction uses Pressed (`5`). Astra's follow-up checked the local SDK declarations at lines 1339 (keyboard event class), 4693 (Pressed `5`) and 4715 (Released `6`), found the mapping valid, found the registration behavior transactional, and reported no other regression. This is a **third, subsequent source finding**, resolved in source; it is not part of the original two Specification P2 count. Native physical shortcut delivery still needs manual confirmation, as recorded in [shortcut evidence](shortcut-native.md).

## Verification boundary

The coordinator's latest final-source [full gate](../../../.artifacts/parallel-rust-only/final/verify-full-r4.log) passed 339 Rust tests, five Python checks, identity, and the packaging/component/asset checks described in [ticket 22](ticket-22.md).

The installed `d9523eb` Release used for the first fifteen-Utility pass predates `df95bd3`. A subsequent corrected Release with synthetic packaging revision `b73c2a7` was actually installed and run: fresh Text Diff editors were empty, one settled comparison recorded, and native WKWebView Cmd-C pasted the selected Unicode word exactly into a GPUI editor; JSON History, Random String controls and the Frappé theme survived isolated relaunch. The latest `b4e17030` Release includes the Carbon correction and passed the r4 full gate; it was installed at the default destination and exercised for identity, isolated JSON persistence, Launcher button, Cmd-W and representative native smoke flows in all fifteen Utilities. This is not an exhaustive native variant matrix. After the coordinator left the same r4 binary on restored synthetic JSON `caffè`, the owner replied “Si sembra funzionare tutto” to a request to test physical Control-Option-Space from another app, another Space/full-screen context and literal Dock return after Cmd-W. [Ticket 22](ticket-22.md) separates this **owner-reported** pass from coordinator-observed native checks and the earlier synthetic-key failure.

Local integration preflight confirmed base `4db64951` is an ancestor of the feature branch. Before this documentation-only closeout, branch `89fd007` and the GitButler workspace had identical tree `c40d748c5244e7babe75922d1069c214e7f6422a`; all 272 tracked worktree contents/modes matched. The coordinator, with explicit user authorization, reconciled a stale pre-relocation index; Git porcelain then reported empty status and GitButler reported zero uncommitted changes on the unpublished local branch. No product file was changed by that repair, and ignored personal files were preserved. The coordinator owns committing this closeout and rechecking final cleanliness; no future hash is asserted here. macOS 14/15 runtime, cross-display/DPI transitions and the accepted IME exception remain outside observed native coverage. No push, pull request, merge or sofui publication is in scope.
