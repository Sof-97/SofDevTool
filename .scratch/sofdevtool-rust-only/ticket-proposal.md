# Approved tickets: Rust-only SofDevTool and sofui

Status: approved breakdown; 22 individual tickets published as ready-for-agent. Implementation has not started.

Source: the [complete specification](spec.md). The owner approved the proposed granularity and blocking edges with “si” on 2026-09-22. This approval covers ticket publication; it does not authorize implementation. The parent specification's scope and status are unchanged.

This document retains its original proposal filename so existing references remain valid. The linked ticket files now own status, acceptance criteria and blocking edges; the summaries below record the approved breakdown.

## Execution boundaries

- The current assignment is documentation only. Do not start implementation, native preview creation, installation or version-control delivery from a triage label alone.
- 01 and 02 expand shared interfaces with working first consumers; 03/04 and 13–18 migrate bounded groups while compatibility adapters keep other consumers working. 19 removes those adapters after migration. Each batch must remain green.
- 20 delivers root relocation with functioning run/install commands. 21 retires Swift only after replacement resource and build ownership is functional.
- The initial frontier is 01, 02, 05, 07, 08 and 17. Technical independence does not authorize concurrent writers in a shared checkout. Begin work only after a separate execution instruction and completion of the selected ticket's blockers.
- Ticket 11 is complete only after the owner accepts the native preview. Ticket 12 and its dependent rollout remain blocked until that acceptance; approving this breakdown does not approve an unseen design.
- Numbers refer to this initiative alone. The historical migration's tickets 01–22 and the parent specification are not closed or re-triaged by publication.

## Published tickets

1. **[Use sofui's owned component interfaces in JSON and the gallery](issues/01-sofui-json-gallery.md)**
   - **Blocked by:** None.
   - **What it delivers:** JSON editing, Copy and keyboard navigation work through documented reusable controls; the gallery uses the same controls without inheriting the application's lifecycle.

2. **[Coordinate JSON and Base64 History with enforced recording recovery](issues/02-history-coordination-recovery.md)**
   - **Blocked by:** None.
   - **What it delivers:** Settings deletion immediately updates open JSON and Base64 History views, and a write failure really pauses recording until a coherent recovery succeeds.

3. **[Propagate History changes through conversion and inspection workspaces](issues/03-history-conversion-workspaces.md)**
   - **Blocked by:** 02.
   - **What it delivers:** Clearing History from Settings immediately invalidates cached entries in eight more Utilities, including workspaces that are open but not visible.

4. **[Propagate History changes through generators, Regex and Text Diff](issues/04-history-generator-comparison-workspaces.md)**
   - **Blocked by:** 02.
   - **What it delivers:** The remaining five Utilities respond consistently to History changes, completing immediate invalidation across the entire catalog.

5. **[Keep Regex responsive and reject obsolete evaluations](issues/05-responsive-regex.md)**
   - **Blocked by:** None.
   - **What it delivers:** A demanding Regex request runs while the app stays interactive, and only the newest complete valid result can appear or enter History.

6. **[Synchronize color controls, output and retained operations](issues/06-synchronized-color.md)**
   - **Blocked by:** 01.
   - **What it delivers:** Each channel adjustment updates the latest requested color consistently in text, swatch, conversions, Copy and settled History.

7. **[Protect generated results when restoring History](issues/07-generator-restore-confirmation.md)**
   - **Blocked by:** None.
   - **What it delivers:** Identifier Generator and Sample Data ask before replacing a different nonempty workspace, even when generation settings match.

8. **[Remember Random String controls independently of History](issues/08-random-string-preferences.md)**
   - **Blocked by:** None.
   - **What it delivers:** Random String reopens with the latest configuration even when History is disabled and no generation has occurred.

9. **[Use generic lists, holds and confirmations for JSON History](issues/09-generic-history-interactions.md)**
   - **Blocked by:** 01, 02.
   - **What it delivers:** JSON History and Settings deletion use reusable list/confirmation controls whose hold progress and cancellation work without application-managed timers.

10. **[Apply observable themes across all open surfaces and editors](issues/10-observable-themes.md)**
   - **Blocked by:** 01.
   - **What it delivers:** Changing Graphite or Catppuccin Frappé updates Workbench, Launcher, Settings and wrapped editors consistently while preserving work.

11. **[Present one native Workbench preview for owner approval](issues/11-native-preview-approval.md)**
   - **Blocked by:** 06, 09, 10.
   - **What it delivers:** The owner can run and review one compact native GPUI design using actual sofui controls before it is applied across the product.

12. **[Apply the approved design to navigation, Launcher and Settings](issues/12-workbench-launcher-settings-redesign.md)**
   - **Blocked by:** 11.
   - **What it delivers:** The redesigned shell supports the familiar catalog/navigation and native Launcher/Settings workflows while retaining every opened Utility Workspace Session.

13. **[Redesign JSON, YAML/JSON, Base64, URL Encoding and JWT](issues/13-text-inspection-workspaces-redesign.md)**
   - **Blocked by:** 03, 12.
   - **What it delivers:** Five text/inspection Utilities use the approved sofui controls from input through diagnostics, Copy and exact History restore.

14. **[Redesign Hashes, Timestamps, Case and Whitespace Conversion](issues/14-conversion-workspaces-redesign.md)**
   - **Blocked by:** 03, 12.
   - **What it delivers:** Four conversion Utilities use the approved interface while preserving their distinct actions, policies and recorded results.

15. **[Redesign Identifier Generator, Random String and Sample Data](issues/15-generator-workspaces-redesign.md)**
   - **Blocked by:** 04, 07, 08, 12.
   - **What it delivers:** All three generators use the approved controls and lists with safe restoration, saved Random String configuration and intact generation behavior.

16. **[Redesign Regex and Color Conversion without regressing corrections](issues/16-regex-color-redesign.md)**
   - **Blocked by:** 03, 04, 05, 12.
   - **What it delivers:** The approved comparison/selection interfaces keep Regex responsive and all color representations synchronized through Copy and History.

17. **[Make Text Diff assets independently maintainable and locally bundled](issues/17-text-diff-asset-pipeline.md)**
   - **Blocked by:** None.
   - **What it delivers:** Text Diff can be rebuilt from retained editable sources and exact dependencies, then run from its local bundle without the Swift wrapper or source checkout.

18. **[Redesign Text Diff and preserve native WebView interactions](issues/18-text-diff-redesign.md)**
   - **Blocked by:** 04, 12, 17.
   - **What it delivers:** The approved Text Diff workspace supports editing, split/unified comparison and exact History restore with correct clipping, focus, scrolling and Copy.

19. **[Finish sofui extraction readiness and remove compatibility scaffolding](issues/19-sofui-independent-consumer.md)**
   - **Blocked by:** 13, 14, 15, 16, 18.
   - **What it delivers:** A separate consumer can run the documented sofui components and gallery without SofDevTool code, resources, persistence or lifecycle.

20. **[Run and install distinct Debug/Release apps from the root workspace](issues/20-root-workspace-packaging.md)**
   - **Blocked by:** 01, 17.
   - **What it delivers:** Root commands build and launch the correctly identified Debug app, package Release and install the intended Release artifact to an overridable destination.

21. **[Retire Swift and make maintained documentation describe the sole Rust app](issues/21-swift-retirement-documentation.md)**
   - **Blocked by:** 20.
   - **What it delivers:** The maintained tree contains one runnable Rust product with accurate setup/ownership guidance and no Swift/Xcode build requirement.

22. **[Verify the installed Release and deliver a clean local branch](issues/22-release-acceptance-local-delivery.md)**
   - **Blocked by:** 19, 21.
   - **What it delivers:** The owner receives a tested Release installation and a clean, committed local branch whose evidence covers the complete agreed product.

## Specification coverage

| Required outcome | Owning slices |
| --- | --- |
| R1: Regex responsiveness and stale work | 05; rechecked after redesign in 16 |
| R2: immediate History invalidation | 02 expands; 03 and 04 complete adoption across all fifteen Utilities |
| R3: recording pause and coherent recovery | 02 |
| R4: synchronized color interaction | 06; rechecked in 16 |
| R5: generated-result restore protection | 07; rechecked in 15 |
| R6: independent Random String preferences | 08; rechecked in 15 and profile acceptance in 20/22 |
| Component-only sofui and extraction | 01, 06, 09, 10; consumer rollout 13–18; contract and independent consumer 19 |
| Approved compact native redesign | 11 owner gate; 12 shell; 13–18 all fifteen Utilities |
| Text Diff provenance and offline resources | 17 and 18 |
| Root workspace, identities and Makefile | 20 |
| Swift retirement, fixtures and current documentation | 21 |
| Actual Release install and clean local delivery | 22 |
| Story 72: specification without implementation | Fulfilled by the planning package and maintained throughout this documentation-only task; not a future implementation ticket |

All implementation stories 01–71 have at least one owner above; story 72 is a constraint on this planning task. Shared invariants from the specification apply to every ticket: strong Utility types, explicit Clipboard access, local-only processing, no personal History inspection/logging/export, meaningful public-behavior tests, and honest native evidence.

## Publication verification

The approved breakdown was published as one file per ticket with the local template, unchanged acceptance criteria and dependency edges, testing guidance and parent-story references. Documentation checks validate numbering, links, complete story coverage and an acyclic dependency graph whose final acceptance includes every preceding ticket. These checks are documentation evidence, not implementation or product-test results.
