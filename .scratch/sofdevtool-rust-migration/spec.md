# SofDevTool Rust and GPUI migration

Status: ready-for-agent

Owner approval: test boundaries and all 22 ticket slices approved in the current conversation.

## Problem Statement

The owner wants one familiar Rust and GPUI stack for developing personal macOS applications with coding agents. SofDevTool currently uses SwiftUI, AppKit, Foundation and a web-backed Text Diff renderer. Its working Utility contracts should survive the rewrite, while its reusable presentation components should become the foundation of the owner's future component library.

The objective is maintainable, reusable development machinery and a useful daily application. Faster development, lower memory use and higher runtime performance are hypotheses, not promised outcomes.

## Solution

Build a Rust version alongside the existing Swift application in the same repository. Preserve the Workbench's information architecture and all fifteen Utilities, using GPUI and an owner-maintained component crate. Deliver working vertical slices, beginning with real text editing, embedded web rendering and macOS Launcher integration. Start all Rust preferences and Utility History afresh in an isolated namespace; never import, clear or rewrite Swift data.

The first release targets Apple Silicon macOS. Retain the existing macOS 14 deployment baseline as a compatibility target, subject to proving the selected dependencies can support it. Runtime claims are restricted to versions actually exercised. A dependency conflict must be reported rather than silently raising the minimum OS.

## User Stories

1. As the owner, I want to use the Developer Toolbox entirely offline, so that developer data stays on my Mac.
2. As the owner, I want the Swift application to remain usable during migration, so that unfinished Rust work does not interrupt daily use.
3. As the owner, I want the Rust application to start with empty History and fresh preferences, so that no migration machinery is needed.
4. As the owner, I want Swift data to remain untouched, so that I can return to the existing application.
5. As the owner, I want Library, Recent and Favorites scopes, so that familiar navigation remains available.
6. As the owner, I want grouped catalog search and keyboard selection, so that I can open a Utility quickly.
7. As the owner, I want an opened Utility Workspace Session retained in memory, so that switching Utilities does not lose my work.
8. As the owner, I want a coherent new visual identity, so that the application feels intentionally designed.
9. As the owner, I want Graphite and Catppuccin Frappé dark themes with Graphite as the fresh default, so that the existing theme choice remains available without importing preferences.
10. As the owner, I want a separate Settings destination, so that changing preferences does not replace my workspace.
11. As the owner, I want explicit Paste and Copy actions with copy feedback, so that Clipboard access remains deliberate.
12. As the owner, I want correct multiline editing, selection, undo and text composition, so that everyday Unicode input works reliably.
13. As the owner, I want a global configurable Launcher shortcut, so that I can find a Utility from another application.
14. As the owner, I want Launcher dismissal to preserve the previously active application, so that opening it need not interrupt my work.
15. As the owner, I want normal Dock, menus, window restoration and explicit Quit behavior, so that the application fits macOS.
16. As the owner, I want settled valid operations recorded once, so that History excludes intermediate edits and invalid results.
17. As the owner, I want History previews and exact restoration without rerunning, so that generated or time-dependent output stays unchanged.
18. As the owner, I want global and per-Utility History controls, so that I choose what is retained.
19. As the owner, I want bounded History and deliberate deletion, so that local retention stays understandable.
20. As the owner, I want a storage failure isolated from the visible result and other Utilities, so that one damaged file does not break the application.
21. As the owner, I want JSON formatting, validation, minification and queries, so that structured text can be inspected locally.
22. As the owner, I want YAML/JSON conversion with explicit fidelity limits, so that unsupported values are not silently changed.
23. As the owner, I want Base64 and URL encoding as separate Utilities, so that alphabet and escaping rules remain clear.
24. As the owner, I want text hashes with explicit legacy labels, so that digest behavior remains predictable.
25. As the owner, I want UUID, ULID and KSUID generation and inspection, so that the existing identifier workflows remain available.
26. As the owner, I want precise timestamps and explicit timezone ambiguity handling, so that conversions do not guess an instant.
27. As the owner, I want JWT decoding without trust claims and with History disabled by default, so that inspection remains distinct from verification.
28. As the owner, I want Regex matches, captures and replacements using the Rust regex dialect, so that the first implementation needs no ICU bridge.
29. As the owner, I want unsupported Regex constructs diagnosed clearly, so that an intentional dialect change is understandable.
30. As the owner, I want deterministic case and whitespace conversion, so that Unicode and indentation behave predictably.
31. As the owner, I want CSS sRGB color conversion and interactive color selection, so that colors can be inspected visually and textually.
32. As the owner, I want fictional JSON/CSV sample rows and cryptographically random strings, so that test data and random text remain local.
33. As the owner, I want split and unified Text Diff with usable selection, scrolling and copy, so that retaining a web renderer does not compromise daily use.
34. As the owner, I want keyboard navigation, visible focus, readable labels and contrast, so that the app remains usable without full assistive-technology certification.
35. As a maintainer, I want strongly typed Utility contracts independent of GPUI, so that agents can change and verify a Utility without changing presentation infrastructure.
36. As a maintainer, I want reusable components independent of SofDevTool, so that a future application can consume them.
37. As a maintainer, I want an executable component gallery, so that component states can be inspected without navigating every Utility.
38. As a maintainer, I want version-pinned dependencies and documented verification commands, so that agent work is reproducible.
39. As a maintainer, I want small end-to-end tickets with explicit blockers, so that each implementation can be reviewed in a fresh context.
40. As the owner, I want a packaged application tested outside the source checkout, so that success is not limited to running from a terminal.

## Implementation Decisions

### Architecture and coexistence

- Use one Cargo workspace nested alongside the existing Swift product. Provide three initial crates: a GPUI-independent Utility core, the SofDevTool application, and an owner-maintained UI library. The gallery is an executable example or binary belonging to the UI library, not a second product architecture.
- The application owns Registry, workspaces, History policy/storage, preferences, platform composition and lifecycle. The core owns Utility-specific requests, results, failures and snapshots. The UI library owns tokens and reusable controls; it must not depend on either SofDevTool crate, Utility IDs, app persistence, or macOS service policy.
- Utility definitions remain source-defined and strongly typed. Erase types only when constructing heterogeneous workspaces. Do not add a universal execute-input interface, dynamic plugin system, general service locator or speculative multi-backend abstraction.
- Reuse the existing Registry, Utility-owned transformation/snapshot and History repository boundaries conceptually. Rust implementation types need not match Swift names.
- Give Rust its own bundle identity, preference domain and Application Support namespace. Use isolated test namespaces. First run is empty; relaunch must preserve newly created Rust data. Clear All can affect only Rust History. Swift builds, source and data remain available through acceptance; automatic uninstall or deletion is excluded.
- Pin the chosen Rust toolchain and GPUI revision/version, commit the lockfile, and document dependency provenance, licenses and native requirements. Choose exact versions during the first technical slice after checking actual compatibility; no speculative dependency list is binding.
- Keep platform-specific code at narrow application-owned boundaries. macOS is the only delivery platform; Windows, Linux and Intel are not required in this increment.

### Component library and presentation

- Build owner-maintained components on GPUI rather than adopting a full external visual system as the application's public API. Targeted dependencies for complex text editing or native interoperability are permitted after evaluation. Do not recreate text editing fundamentals merely to avoid a dependency.
- Shadcn is a reference for composability and readable source. Initial distribution is a local versioned crate, not copied components, a registry service, scaffolding CLI or published package. Extract the library to another repository when a second consuming app makes its reusable boundary concrete.
- Grow components from working slices: semantic theme tokens, buttons, labeled inputs, multiline editor, select/segmented controls, panels, lists, dialogs, diagnostics and copy feedback. Add color and schema-editing primitives when their Utility needs them. No separate large component-library phase precedes useful app behavior.
- The gallery must use the actual exported components and demonstrate their applicable normal, focused, disabled, invalid, loading and empty states. It must build without depending on the SofDevTool application.
- Preserve compact toolbar, grouped catalog, flexible selected workspace and collapsible trailing History. Recent/Favorites are catalog scopes, not dashboards. Preserve separate Settings and per-Utility modes rather than a universal form.
- Use a new coherent visual treatment while retaining these flows. Preserve the current two dark theme choices, default and attribution; geometry and component styling may evolve. Theme changes must not recreate sessions, record operations or change Recents.
- Required usability covers keyboard navigation, visible focus, readable labels, non-color-only errors and legible contrast. Full VoiceOver parity and exhaustive OS accessibility-setting adaptation are deferred. Keep semantic metadata where readily supported, but do not treat a passing screen-reader audit as a release gate.

### Native behavior and embedded rendering

- Prove real multiline Unicode editing, selection, clipboard, undo/redo, composition/IME, scrolling and focus traversal in the first running app. GPUI drawing alone is not proof of editor usability.
- Prove an embedded web diff in the same GPUI window early, including resize, clipping, display scaling, selection/copy, scrolling and focus handoff. Retain the renderer's web assets and behavior behind an application-owned Text Diff boundary. The Swift wrapper is not a required retained dependency.
- Package renderer assets locally; allow no remote script/font fetching or payload-bearing network traffic. Handle readiness idempotently, reject stale request callbacks, surface renderer failure, and retain the Unicode bridge regression and disclosed whole-line fallback for complex emoji until equivalent behavior is demonstrated.
- If a lightweight Rust/native WebView bridge cannot meet the proof criteria, record the concrete failure and alternatives before broad migration. Do not silently switch to a native diff rewrite or turn the whole app into a WebView shell.
- Global shortcut registration is separate from in-window keybindings. Preserve Control-Option-Space as default; registration failure keeps the last working shortcut and produces an inline diagnostic.
- Launcher searches the shared Registry, is transient and nonactivating, appears on the active display/current Space including over a full-screen app, and dismisses via Escape, click-away or repeated shortcut. Selection activates the Workbench and opens the Utility. Closing all regular windows keeps the process alive; Dock activation restores the Workbench; Quit ends the process. No login item or persistent menu-bar icon is added.

### Operations, History and privacy

- Preserve neutral empty states, precise diagnostics, explicit generation/expensive actions, task-appropriate debounce and no stale visible output for invalid current input. Only the current workspace revision may publish an asynchronous result or snapshot.
- Clipboard reads/writes are explicit; never monitor or classify Clipboard contents. No accounts, telemetry, sync or network-backed Utility operations are introduced.
- Retain the newest 25 entries per Utility, globally enabled by default and independently configurable per Utility. JWT History defaults off. Turning recording off preserves existing entries and per-Utility settings.
- Use new versioned Rust serialization with stable entry IDs, timestamps, Utility identity, snapshot version and Utility-owned payload. There is no Swift decoder or migration path. Preferences never contain Utility inputs/results.
- Store one versioned JSON History file per Utility with atomic replacement and serialized writes per Utility. Preserve the last valid file on failure, show a nonmodal warning, and pause that Utility's recording until successful retry or relaunch. Malformed files are isolated and not silently overwritten; unknown snapshots remain listed as unavailable.
- One settled valid operation records at most once. Deliberate repeated generation records separately. Preview and restore never execute or record; restore warns only before replacing a different nonempty session.
- Preserve one-second pointer hold for Clear Utility and two seconds for Clear All, cancellation on early release/pointer exit/Escape, and a normal confirmation dialog for keyboard activation. Deletion preserves current workspace content and is restricted to Rust History, including unknown Utility files there.
- Never log, index, sync or export History payloads. Do not add encryption/Keychain machinery or persistent workspace autosave.

### Utility behavior and approved changes

The existing product contracts and independent test fixtures remain the behavioral baseline except for the explicit changes below. Dependency choices tied to Swift are replaced, not inherited as requirements.

| Utility | Required behavior |
| --- | --- |
| JSON | Format, minify, validate, configurable indentation and sorting, JSON Pointer and existing simple dot/bracket query paths; preserve numeric meaning and array order; never repair invalid JSON silently. |
| YAML / JSON | Single YAML 1.2 Core-oriented document, anchors/aliases, string-keyed mappings and JSON fidelity checks; reject duplicates, nonfinite/unrepresentable values, unsupported tags and multiple documents; disclose loss of comments/formatting/alias identity. |
| Base64 | Encode/decode UTF-8, standard and URL-safe alphabets, padding control, strict invalid-input diagnostics. |
| URL Encoding | Separate Path Segment and Query Value modes; strict UTF-8 percent decode, uppercase percent bytes, spaces as %20, literal plus preserved on decode. |
| Hashes | Explicit SHA-256/384/512, SHA-1 and MD5 over exact UTF-8; hex casing/Base64; legacy labels; explicit empty-byte hashing is valid. |
| Identifier Generator | UUID v1/v3/v4/v5/v6/v7, validation/normalization/options; ULID random and process-local monotonic; KSUID random and explicit ordered batches with 65,536 cap; inspection and exact restore. |
| Timestamps | Auto/manual Unix seconds/milliseconds, ISO 8601 and named-zone local time; reject ambiguous inference and repeated/nonexistent DST wall times; fixed captured instant on restore. |
| JWT Decoder | Separate readable header/payload, opaque signature, precise segment diagnostics, no keys or trust/claim-validation assertions; History initially off. |
| Regex | Rust regex crate dialect, supported flags, all matches/captures and replacement preview; unsupported look-around/backreferences diagnosed; visible engine/dialect label and replacement syntax guidance. |
| Case Conversion | Existing nine styles using one visible deterministic segmentation policy; Unicode/grapheme-safe results independent of system locale. |
| Whitespace Conversion | Existing nine separate actions; LF default; tab width 1–8, default 4; tab-stop expansion, leading-indent-only spaces-to-tabs, explicit line-ending and dedent behavior. |
| Color Conversion | Bounded sRGB HEX/RGB(A)/HSL(A), deterministic rounding and alpha, synchronized interactive color selection and invalid-text handling; no expanded color-space scope. |
| Sample Data | Typed reorderable fictional fields, JSON/CSV, default ten rows, maximum 1,000 rows and 50 fields; strict schema validation and correct CSV quoting; no external data. |
| Random String | System cryptographic randomness, unbiased alphabet sampling, existing length/count/character controls and exclusions, honest entropy, per-item Copy and Copy All. |
| Text Diff | Split/unified views, usable scroll/selection/copy, renderer failure/loading states, exact old/new text snapshot, UTF-8 and complex-emoji correctness. |

- Regex intentionally replaces ICU rather than emulating it. There is no ICU dependency or promise of future ICU support. Keep the Utility boundary replaceable without implementing multiple engines now. Use bounded pattern compilation/input/match/capture/replacement output policies and bounded off-UI work. Do not claim an interruptible engine or hard deadline the selected crate cannot provide. Discard stale work, check cancellation between controllable stages and do not publish partial results as success.
- Retain the existing timestamps inference policy: signed integers of at most ten digits mean seconds, exactly thirteen mean milliseconds, eleven/twelve require manual selection; fractional seconds require explicit seconds mode. ISO Auto requires an offset or Z.
- Every Utility includes Registry metadata, session, diagnostics, explicit Clipboard actions, snapshot recording according to policy, preview/restore and meaningful tests. An unfinished Utility must not appear as a working catalog item.

## Testing Decisions

- Approved testing boundary: test the highest useful existing conceptual seams—Utility request/result/snapshot contracts; application session plus temporary storage; and the real macOS application for integration behavior that cannot be established in core tests. The owner approved these boundaries.
- Reuse published vectors and hand-reviewed fixtures from the Swift suites as independent oracles. Do not copy implementation algorithms into expected-value calculations or assert private component nesting.
- Use fixed clocks/timezones, deterministic randomness and controlled completion order. Test invalid/empty/Unicode/boundary cases, exact restore, deliberate repeats and rejection of obsolete asynchronous publications.
- Test the shared recording/storage path through sessions and temporary directories, including retention, atomic failure, corruption isolation, switches, preview/restore and data-namespace separation. Keep Utility-specific domain tests with their owner.
- Component tests cover public interactions; the gallery provides visual/manual evidence. Test actual controls in app flows rather than relying on screenshots alone or testing every style value.
- Use focused app tests where the chosen GPUI tooling supports them, plus documented native manual scenarios for global shortcut, active Space/full-screen, Dock lifecycle, IME and WebView focus/copy. Any automation gap must be named; unit tests or compilation cannot substitute for native evidence.
- Establish one documented Rust formatting/build/test gate and an extended release/native verification workflow alongside the existing Swift commands. Each ticket runs the appropriate current Rust gate; native changes add their focused real-app scenarios. Record commands, host OS, architecture, toolchain and evidence paths. Do not run the entire Swift suite for documentation-only changes.
- First technical milestone passes only after editor, embedded diff and Launcher are usable together in one pinned GPUI application. Broad Utility migration depends on that milestone.
- Release requires all fifteen Utility contracts, shared History behavior, isolated fresh start and subsequent persistence, component-gallery independence, packaged offline operation from outside the checkout, Debug/Release identity, and honest OS compatibility evidence. No numeric speedup or memory target is asserted without measurements.

## Out of Scope

- Importing Swift History, preferences, recent selections or favorites; deleting old data or uninstalling Swift automatically.
- Implementing product code during this planning task.
- Windows/Linux/Intel delivery, full VoiceOver parity, accessibility certification or exhaustive platform-setting parity.
- ICU integration, multiple Regex engines, Swift Regex compatibility, new Utility families, file/batch workflows and persistent workspace autosave.
- A separate component repository now, public crate publication, a shadcn-style copying CLI, universal component schema or full external design-system adoption.
- Native Text Diff replacement unless new evidence requires a separately agreed change.
- Accounts, telemetry, cloud/network services, plugin marketplaces, public distribution, notarization or App Store delivery.

## Further Notes

- This is the approved implementation specification, not runtime evidence. Product choices, test boundaries and ticket granularity were approved in the current conversation. Execution is handed off to the new orchestrator session; no product code was implemented during planning.
- This migration-specific contract supersedes Swift technology choices, ICU dialect, full accessibility parity and data continuity only for the Rust product. Existing Swift planning records remain historical and must not be rewritten as if their implementation had changed.
- Behavioral references: [domain glossary](../../CONTEXT.md), [architecture handoff](../sofdevtool/implementation-handoff.md), [remaining Utility contracts](../sofdevtool/remaining-utilities-implementation-spec.md), [existing themes and Workbench](../sofdevtool/specs/precision-workbench-and-themes.md).
- [Ticket index](ticket-review.md) lists approved slices and blocking edges. Individual tickets are published in the local issues directory as ready-for-agent.
