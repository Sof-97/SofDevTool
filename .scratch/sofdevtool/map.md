Label: wayfinder:map

## Destination

A decision-complete product brief and technical architecture for SofDevTool, followed by a compiling and verified macOS scaffold with navigation, search, settings, shortcuts, tests, and representative JSON, Base64, Identifier Generator, and Random String Utilities.

## Notes

- Product boundary: a private, completely local Developer Toolbox for its owner; source may be shared informally with colleagues later, but this is not a customer-facing product.
- Cost boundary: no subscriptions, hosting, paid APIs, or required Apple Developer Program membership. A membership may be reconsidered only after the personal app proves valuable.
- Platform boundary: Apple Silicon on the current and previous two macOS generations.
- Native behavior and a polished macOS UI matter more than preserving any particular programming language. Favor technologies that agents can navigate, modify, and verify reliably.
- Core Utilities work offline. Preferences and configurable Utility History may persist locally; history defaults on with explicit privacy controls.
- Use `domain-modeling`, `grilling`, `research`, `prototype`, and `codebase-design` as appropriate while resolving tickets.
- Execution exception: Wayfinder normally stops at decisions, but this effort explicitly carries through creation and verification of the initial scaffold after the decisions are closed.

## Decisions so far

- [Choose the macOS application technology stack](issues/01-choose-macos-technology-stack.md) — Use Swift 6, SwiftUI, and narrow AppKit adapters on macOS 14+ for the smallest native, agent-verifiable stack; keep Tauri 2 only as the rejected web-shell fallback.
- [Define first-release Utility contracts](issues/02-define-first-release-utility-contracts.md) — Fix the shared interaction, privacy, defaults, and exact boundaries for the first-release catalog and replace the scaffold's UUID slice with a complete UUID/ULID/KSUID Identifier Generator.
- [Prototype the main window and Utility Launcher](issues/03-prototype-main-window-and-launcher.md) — Use the Workbench/keyboard-first layout: a scoped, searchable grouped Utility list beside a persistent workspace, plus a transient global Launcher and separate Settings destination.
- [Decide native integration boundaries](issues/04-decide-native-integration-boundaries.md) — Keep a Dock-based dark-only app alive after window closure, with a configurable global Launcher, explicit clipboard access, native accessibility behavior, and no extra system surfaces in the scaffold.
- [Design the Utility module interface](issues/05-design-utility-module-interface.md) — Use compile-time, strongly typed Utility modules behind a small type-erased registry seam, with Utility-owned workspaces and versioned History snapshots plus narrow Clipboard and History adapters.
- [Design Utility History storage and privacy](issues/06-design-history-storage-and-privacy.md) — Keep a bounded per-Utility local JSON history with explicit recording controls, shell-owned preview and restore, accessible hold-to-clear actions, and isolated failure handling without special protection machinery.
- [Define the verification and agent workflow](issues/07-define-verification-and-agent-workflow.md) — Use a three-target native Xcode project with one authoritative local verification gate, contract-traceable Utility tests, focused UI and manual accessibility checks, honest per-macOS runtime evidence, and minimal pinned dependencies.
- [Validate PierreDiffsSwift for Text Diff](issues/10-validate-pierre-diff-integration.md) — Use a patched, exact-pinned Pierre renderer behind a replaceable module, with emoji-safe highlighting, offline/accessibility/performance gates, and complete bundled-code notices.
- [Write the product and architecture handoff](issues/08-write-product-and-architecture-handoff.md) — Use one linked implementation contract for scaffold scope, architecture seams, exact representative behavior, local persistence, native UX, and verifiable completion.
- [Build and verify the initial scaffold](issues/09-build-and-verify-initial-scaffold.md) — The native macOS scaffold, representative Utilities, Launcher, Settings, History, shortcut and lifecycle behavior, automated gates, and macOS 26 owner checks are complete; macOS 14 and 15 retain explicit build-only compatibility evidence until runtime hosts are available.

## Not yet specified

- Smart clipboard recommendations: exact content-detection rules, ambiguity handling, privacy feedback, and launcher UX should be revisited after the ordinary Utility Launcher is validated. The initial architecture must leave a clean seam, but the feature is not part of the scaffold.
- The post-launch catalog beyond the agreed initial Utilities should be shaped by real personal usage rather than predicted now.

## Out of scope

- Public distribution, the Mac App Store, notarization, and a polished colleague-distribution workflow are deferred until the personal app proves useful.
- Accounts, cloud synchronization, telemetry, hosted services, and paid APIs.
- A third-party plugin marketplace or runtime-loaded plugin system; new Utilities are source-code modules.
- Replacing an IDE, terminal, Git client, database browser, API client, or full browser DevTools suite.
- Intel Mac support.
