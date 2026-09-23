# Complete the Rust-only SofDevTool and prepare sofui for independent release

Type: task
Status: ready-for-agent
Blocked by: None for preparation; owner approval of the native preview is required before the full visual rollout.

Scope agreed: 2026-09-22, through the owner interview and final scope confirmation.
Execution state: specification only. The owner's latest instruction is to write this complete specification, not to start implementation. Triage status does not override that instruction.

## Problem Statement

The owner wants one finished Rust and GPUI Developer Toolbox, with a repository that clearly contains and supports that product. The migration currently coexists with the previous Swift application, Xcode configuration, old build commands, duplicated assets and superseded planning material. Root documentation still directs contributors toward the old product. Although the Rust catalog is implemented and its automated gate passes, review found six behavioral defects that undermine responsiveness, History consistency, color interaction, safe restoration and preference continuity.

The current UI crate has useful dependency separation but is still shaped around SofDevTool. Some product vocabulary and policy appear in presentation components; process startup is exposed through the library; consumers must repeat interaction machinery; and theme changes require caller-managed redraws. The owner intends to release this crate separately as a personal GPUI component library named **sofui**. It needs a clear, enforceable component-only responsibility now, while useful application work continues.

The existing interface also needs a deliberate visual redesign. The owner wants a compact, polished native Workbench, preserving familiar navigation, with one real GPUI preview reviewed before the design is applied throughout the application. Finally, the owner needs understandable run and install commands, separate Debug and Release identities, correct version information, and a clean local branch ready to merge.

## Solution

Complete the Rust application as the sole product in the repository. Correct all six review findings, preserve the complete fifteen-Utility behavioral contract, move the Cargo workspace to the repository root, and retire the Swift implementation and obsolete development machinery. Keep editable provenance and verification for the embedded Text Diff renderer, reusable icon artwork, independent test vectors and useful current evidence.

Develop sofui as an independently versioned GPUI library whose responsibility is reusable component presentation and interaction. SofDevTool remains the first consumer. Application policy, Utility execution, data persistence and native application lifecycle remain outside sofui. Prove independent consumption locally; actual repository extraction and publication happen later when a second real application makes the reusable interface concrete.

Deliver the agreed compact native redesign in two stages: first a functioning native preview of the shell and representative components, then the full application rollout after the owner reviews it. Preserve Library, Recent and Favorites, the Utility sidebar, central workspace, trailing collapsible History, separate Settings, and the global Utility Launcher.

The final product is **SofDevTool 0.2.0**, consuming **sofui 0.1.0**. A root Makefile provides documented development, verification, packaging and installation commands. The final implementation includes actual installation and launch from the owner's Applications directory, tested using isolated data. Delivery stops at a clean, committed, merge-ready local branch; no push or pull request is included.

## User Stories

1. As the owner, I want the repository to contain only the supported Rust application, so that I never have to choose between two implementations.
2. As a contributor, I want the Cargo workspace at the repository root, so that the project layout immediately identifies the active product.
3. As a contributor, I want the root README to describe the actual Rust application, so that its setup and usage instructions work from a fresh checkout.
4. As a coding agent, I want one current agent guide, so that obsolete Swift instructions do not steer new changes.
5. As the owner, I want obsolete Swift code, Xcode targets and executable spikes removed, so that maintaining the application does not require the old stack.
6. As a maintainer, I want useful independent test vectors preserved, so that deleting the old implementation does not discard behavioral evidence.
7. As a maintainer, I want the Text Diff renderer's editable sources, exact dependencies and build process retained, so that its bundled JavaScript remains maintainable.
8. As the owner, I want existing data from the old application left untouched, so that source cleanup does not destroy personal information.
9. As the owner, I want the installed Release app to be named SofDevTool, so that the product no longer looks like a migration experiment.
10. As the owner, I want the Release bundle identified as com.gerardocalia.sofdevtool, so that the application uses my chosen identity.
11. As a developer, I want Debug to have a separate identity and data namespace, so that development cannot change Release preferences or History.
12. As a developer, I want Debug to be recognizable in the Dock and inside the application, so that I can tell which build I am using.
13. As the owner, I want the established app artwork retained, so that the new implementation preserves the recognizable product icon.
14. As the owner, I want an intentional semantic version and build revision, so that a reported problem can be traced to the installed source.
15. As a component-library maintainer, I want sofui versioned independently, so that an application release does not dictate a library release.
16. As a developer, I want one command to build and launch the Debug app, so that development uses a correctly identified native bundle.
17. As the owner, I want one command to install Release, so that using the application does not depend on remembering packaging details.
18. As the owner, I want the installation destination overridable, so that I can choose where the app lives without editing a script.
19. As the owner, I want the packaged app to run outside the checkout and offline, so that it is a normal local macOS application.
20. As a reviewer, I want a clean local branch with reviewable commits and accurate verification evidence, so that merging the work is straightforward.
21. As the owner, I want a compact native visual design with clear typography and restrained borders, so that a dense Utility workspace remains readable.
22. As the owner, I want to review a real GPUI preview before the full redesign, so that I can assess its interaction and density early.
23. As the owner, I want the established navigation retained, so that a new visual treatment does not require relearning the application.
24. As the owner, I want Library, Recent and Favorites to remain catalog scopes, so that finding Utilities stays quick and predictable.
25. As the owner, I want grouped search and keyboard selection, so that I can open a Utility without relying on a pointer.
26. As the owner, I want opened Utility Workspace Sessions preserved while switching Utilities, so that navigation does not discard work.
27. As the owner, I want a collapsible trailing History inspector, so that retained operations remain accessible without permanently consuming workspace area.
28. As the owner, I want Settings to remain separate from the active workspace, so that configuring the application does not replace my work.
29. As the owner, I want the global Launcher to preserve its native activation and dismissal behavior, so that it remains useful from other applications and Spaces.
30. As the owner, I want Graphite and Catppuccin Frappé to remain available, so that the redesign preserves the agreed theme choices.
31. As the owner, I want a theme change reflected in all open surfaces and text editors, so that the application has one coherent appearance.
32. As the owner, I want visible focus, readable labels and non-color-only diagnostics, so that dense controls remain usable.
33. As the owner, I want explicit Clipboard actions and clear copy feedback, so that access to my Clipboard stays deliberate.
34. As the owner, I want expensive Regex work to run without blocking interaction, so that entering a demanding pattern does not freeze the application.
35. As the owner, I want obsolete Regex work discarded, so that changing or clearing input cannot publish an old result afterward.
36. As the owner, I want Regex limits and diagnostics to describe actual guarantees, so that I am not misled by an inaccurate performance claim.
37. As the owner, I want clearing History to remove its entries from every affected view immediately, so that deleted records cannot remain selectable or restorable.
38. As the owner, I want History deletion to preserve my active workspace content, so that removing retained records does not erase current work.
39. As the owner, I want a failed History write to pause recording for that Utility, so that the application behaves as its warning says.
40. As the owner, I want successful History recovery reflected consistently, so that recording does not continue while still being labelled paused.
41. As the owner, I want corrupt or unavailable History isolated per Utility, so that one failure does not prevent other Utilities from working.
42. As the owner, I want global and per-Utility recording controls preserved, so that I remain in control of retention.
43. As the owner, I want color selection synchronized with text, preview, Copy and History, so that each representation describes the same current color.
44. As the owner, I want consecutive color adjustments to accumulate correctly, so that repeated clicks operate on the latest state.
45. As the owner, I want confirmation before restoring over a different nonempty generated result, so that unchanged settings do not hide the fact that my output will be replaced.
46. As the owner, I want restoration to reproduce captured output without running a generator or clock again, so that a retained operation stays exact.
47. As the owner, I want Random String controls saved independently of History, so that my preferred generation configuration survives relaunch even when recording is disabled.
48. As the owner, I want generated strings excluded from ordinary preferences, so that saving controls does not create another storage path for output.
49. As the owner, I want all fifteen Utilities to retain their modes, diagnostics and snapshots, so that cleanup and redesign do not remove working capabilities.
50. As the owner, I want neutral and invalid inputs handled clearly, so that stale output is never presented as a current valid result.
51. As the owner, I want settled valid operations recorded once and deliberate repeated generation recorded separately, so that History reflects meaningful operations.
52. As the owner, I want JWT History disabled by default, so that readable token contents are retained only when I enable that policy.
53. As the owner, I want the embedded Text Diff to preserve Unicode, selection, copy, scrolling and resize behavior, so that it remains useful during everyday editing.
54. As the owner, I want all Utility processing and renderer resources local, so that the app has no account or network dependency.
55. As a component-library maintainer, I want sofui to contain only reusable UI components and component logic, so that it can later support unrelated GPUI applications.
56. As a component-library maintainer, I want application lifecycle and persistence outside sofui, so that consuming it does not adopt SofDevTool's runtime or storage policy.
57. As a component-library maintainer, I want the component-only rule written explicitly in the agent guide, so that future changes preserve the intended separation.
58. As a component consumer, I want stable identities and predictable focus behavior, so that repeated labels and normal keyboard interaction do not cause failures.
59. As a component consumer, I want text APIs to distinguish silent assignment from user editing, so that programmatic changes have understandable event and undo semantics.
60. As a component consumer, I want reusable hold timing and cancellation behavior, so that each application does not rebuild the same interaction state machine.
61. As a component consumer, I want generic selectable lists and confirmation presentation, so that product-specific History policy is supplied by my application.
62. As a component consumer, I want application-wide observable themes and customizable tokens, so that I can use sofui with my own visual identity.
63. As a component-library maintainer, I want new controls driven by real application needs, so that the library grows without a speculative catalog.
64. As a component-library maintainer, I want the complex text-editing dependency kept behind owned interfaces, so that I do not need to reimplement editing fundamentals.
65. As a component consumer, I want an independently runnable gallery of the actual exported controls, so that I can inspect their supported states and interactions.
66. As a component-library maintainer, I want public interaction tests, so that behavior is protected without freezing private layout details.
67. As a component-library maintainer, I want an independent consumer check, so that extraction readiness is demonstrated rather than inferred from a dependency diagram.
68. As a contributor, I want deterministic regression tests at meaningful seams, so that the six review defects cannot return unnoticed behind a green domain suite.
69. As a reviewer, I want native evidence distinguished from builds and unit tests, so that verification claims accurately describe what was exercised.
70. As the owner, I want the final Release installation tested with isolated data, so that verification does not alter personal History or preferences.
71. As the owner, I want old OS and IME limitations stated honestly, so that unsupported evidence claims do not become accidental promises.
72. As the owner, I want this specification delivered without product implementation, so that the agreed work is recorded before execution starts.

## Implementation Decisions

### Scope and source precedence

- This specification governs completion of the existing Rust port. It supersedes the earlier requirements to retain Swift source through acceptance, keep a nested Rust workspace, use migration-era product names, and postpone the present visual redesign.
- The fifteen Utility contracts, local-only product model, strong Utility types, explicit Clipboard policy, configured History semantics, native Launcher behavior and macOS deployment baseline remain in force except for the corrections made explicit here.
- The latest owner instruction limits the present task to specification and local tracker publication. Product implementation, prototype creation, installation and version-control delivery described below are future work.
- The full redesign has a mandatory owner-review checkpoint. Overall scope approval does not constitute approval of an unseen preview.

### Rust-only repository and documentation

- Make the Rust Cargo workspace the root project. Retain three meaningful modules as crates: the application, the GPUI-independent Utility core, and sofui. Keep application and core types strongly owned; type erasure remains confined to heterogeneous workspace construction.
- Remove the Swift application and test targets, Xcode configuration and package-resolution machinery, Swift-specific formatter/build/release tooling, vendored Swift wrappers, and obsolete executable spike copies. Remove superseded migration WIP patches and planning instructions from the maintained tree. Git history is the reference for retired work; a checked-in historical archive is not required.
- Before removing old sources, preserve independent behavioral fixtures still needed by the Rust contract suite, the established icon artwork, and the editable Text Diff asset pipeline and its provenance. Do not preserve a second runnable product just to retain these resources.
- Keep only current product contracts, maintainership guidance and useful verification evidence. Dated evidence may remain clearly labelled as historical evidence of a specific candidate; it must not be presented as verification of the final changed product. Repair or replace retained links that would otherwise point into deleted material.
- Rewrite the root README around the actual fifteen-Utility Rust application, prerequisites, component ownership, development commands, preview review, packaging, installation, data namespaces and known evidence limits.
- Rewrite the root agent guide to identify the current source ownership and verification commands. It must explicitly state that sofui will later be released separately and may contain only reusable UI components and component logic. It must assign Utility execution, domain validation, persistence, History policy and application lifecycle to application/core ownership.
- Retain the domain glossary. Document the current architecture without leaving incompatible Swift-only guides as active prerequisites.
- Preserve pinned toolchain and dependency resolution. Relocation alone is not an instruction to upgrade dependencies. The observed baseline is Rust 1.98.1 with GPUI 0.3.6 and the wrapped component dependency 0.6.6; necessary changes require concrete compatibility evidence and an updated lockfile/provenance record.
- A Rust-only product still uses macOS native frameworks and the retained embedded JavaScript renderer. Removing Swift source does not require removing AppKit interoperability, WKWebView or non-Swift renderer build tools.

### Application, core and sofui ownership

| Owner | Responsibilities |
| --- | --- |
| Application | Registry composition, Workbench, Launcher, Settings, concrete Utility workspaces, application session coordination, History coordination and repository, preferences, explicit Clipboard adapters, native window/shortcut behavior, process lifecycle, packaging identity and persisted theme choice. |
| Utility core | Strongly typed Utility requests, evaluations, failures and snapshots; transformations, formatting and parsing; Utility-specific limits and validation; clock/random seams where needed; deterministic contract tests and shared revision/snapshot bookkeeping. |
| sofui | Reusable rendering, layout, semantic theme tokens, component state, focus, keyboard/pointer interaction, selection, editing/undo integration, hold timing/cancellation, generic feedback/diagnostic presentation and reusable visual control composition. |

- sofui must not depend on the application or Utility core, know Utility identities or snapshot formats, read or write application files/preferences, decide retention/recording/restore policy, register the product's global shortcut, or own process startup and Dock lifecycle.
- Put GPUI platform application creation and process lifecycle hooks in the application and gallery entrypoints. Library initialization and mounting may remain in sofui where they are required to correctly initialize its component and editor infrastructure.
- Keep the existing complex text-editing engine behind sofui-owned public interfaces. This work does not replace it with a custom editor or adopt the external dependency's entire visual system as the application's public interface.
- Keep the associated request/result/snapshot types concrete. Shared coordination must not introduce a universal erased-input execution interface, plugin loader or general service locator.
- Centralize repeated History coordination in the application. Its shared interface owns entry invalidation, selection validity, persistence status and common actions. Utility-owned decoding, preview content and restore semantics remain with the owning Utility.
- A generic list or inspector may accept rows, stable presentation IDs, labels, selection, status and actions. Product concepts such as History, unavailable snapshots and the retention quota are supplied by the application; sofui does not hardcode their wording or meaning.

### sofui public interactions and extraction readiness

- Name the crate, documentation and gallery consistently as sofui. Use product-independent component action namespaces and identifiers. Assign it independent version 0.1.0, rather than requiring future releases to inherit the application's version.
- Make stable control identity explicit and independent of visible labels. Repeated labels and changing labels must be safe. Focus must survive redraws, and supported controls must support their normal keyboard activation without each consumer reconstructing an undocumented interaction recipe.
- Define text assignment versus user editing in the public interface. Silent initialization/restoration and user-style edits must have documented, distinct notification and undo behavior. Consumers must be able to deliberately request the appropriate semantics without naming dependency-internal types.
- Provide reusable hold behavior including duration, progress, completion and cancellation on early release, pointer exit and Escape. Preserve keyboard activation through ordinary confirmation. The application supplies the destructive action and product-specific duration; the library owns reusable timing and interaction mechanics.
- Extract controls supported by real application use, including selection/segmented controls, numeric stepping, generic selectable lists, confirmation presentation and color-channel selection where the approved design needs them. Keep schema types, generation rules and Utility-specific parsing outside the library.
- Preserve useful empty, loading, invalid, disabled, selected, focused and copy-feedback states. Do not build a broad unused catalog or a generic schema-driven application framework.
- Use an observable application-wide theme mechanism with customizable semantic tokens. Graphite and Catppuccin Frappé remain presets; Graphite remains the fresh default. Theme changes update all affected open windows and the wrapped editor appearance consistently. Persisted choice belongs to the application.
- Remove the assumption that the consumer is single-window. Independent simultaneous per-window themes are outside this increment; consistent updates across Workbench, Launcher, Settings and gallery remain required.
- Theme changes preserve Utility Workspace Sessions, input, results, History, Recents and focus context where applicable. They must not cause new Utility Operations or recreate workspace entities merely to refresh colors.
- Keep an executable gallery that consumes the actual public components, builds without the application/core, and demonstrates supported states plus keyboard and pointer interactions. Document initialization, mounting, identity, focus, text events, themes and the dependency/license requirements.
- Demonstrate a standalone consumer using sofui without compiling or importing the SofDevTool application/core or reading product resources. If a temporary extraction is used, supply the dependency metadata currently inherited from the workspace. A no-app dependency graph alone is insufficient evidence of independent consumption.
- Keep sofui in this repository for now. Extraction to another repository and separate release remain a later step when a second real application validates the interface. Do not publish a crate or create a separate project as part of this work.

### Required correction R1: responsive, revision-safe Regex work

- The current implementation delays on a background timer but evaluates synchronously inside a UI entity update. Move actual pattern compilation, matching, capture collection and replacement work off the UI thread; moving only the debounce delay does not satisfy the requirement.
- Use bounded background work and bounded pending requests. New input supersedes obsolete work without creating an unbounded queue or thread/task fan-out. Check cancellation or obsolescence between stages under application control, and reject stale completion before publishing either visible state or History.
- Clearing input, restoring a snapshot, or submitting a newer request invalidates earlier work. Pending state must not present a previous output as current. Only a complete, current, valid result may create one snapshot.
- Preserve the existing Rust regex dialect, flags, unsupported-construct diagnostics and replacement guidance. Remove the blanket claim that the whole matching/iteration workflow is linear time. Do not claim a hard deadline or interruption of an individual engine call when the dependency cannot provide it.
- Preserve explicit resource refusals without partial successful results. Current budgets are 16 KiB pattern text, approximately 1 MiB compiled program, 1 MiB test text, 256 KiB replacement template, 10,000 matches, 50,000 capture slots and 2 MiB replacement output. A justified policy adjustment must be documented rather than silently expanding or dropping limits.
- Acceptance includes a demanding pattern/input, continued UI interaction while it is being evaluated, and a newer request winning over its obsolete completion. The review's measured delay is reproduction evidence, not a new arbitrary latency service-level objective.

### Required correction R2: History deletion invalidates live views

- Successful Clear Utility removes persisted and in-memory retained entries for that Utility. Successful Clear All does the same for every affected Utility, including already-open but currently hidden workspaces and unknown Utility files within the application namespace.
- Open History inspectors update immediately without requiring another operation or relaunch. Deleted entries cannot remain previewable or restorable through stale selection, pending confirmation or cached state.
- Preserve the active Utility Workspace Session's controls, input, diagnostics and result. Cancelling a pending restore of a deleted record must not clear the actual workspace.
- Route changes through shared application-owned History coordination rather than independent ad hoc reload logic repeated in all fifteen workspaces. Notifications must reach every affected subscriber without requiring each subscriber to understand storage layout.
- Handle partial or failed deletion honestly: show the failure and reconcile views with what actually remains. Do not announce that all entries were cleared if a storage operation failed.

### Required correction R3: enforce History recording pause and recovery

- A failed write preserves the last valid file and visible Utility result, produces a nonmodal warning, and actually stops ordinary recording for that Utility. The paused state is enforced by the application recording path, not only displayed in Settings.
- Continued edits and operations while paused must not silently resume disk writes. Other Utilities remain operational according to their own recording policies.
- Provide a coherent retry/recovery path consistent with the existing retry-or-relaunch contract. Clear the paused state only after successful recovery; an unsuccessful retry remains paused and visible. Recovery must not duplicate an already retained operation or publish an invalid/stale operation.
- Relaunch resets the in-memory pause state, but does not repair, overwrite or hide corrupt storage. Malformed files remain isolated and deliberately removable. Ordinary successful writes must not leave contradictory paused indicators behind.
- Keep global and per-Utility recording choices independent. Disabling recording preserves retained entries. A storage failure and a user-disabled recording policy remain distinguishable states.

### Required correction R4: synchronized color interactions

- Color selection updates the complete current state: editable representation, numeric channels, swatch, converted outputs, Copy content and the eventual settled snapshot.
- Consecutive adjustments accumulate from the latest requested color, including when earlier evaluation is pending. They must not repeatedly derive from an older settled result.
- Use the text-control interface's explicit event semantics or schedule evaluation deliberately; do not rely on silent assignment emitting a user change.
- Preserve invalid-text diagnostics and the policy that invalid current input cannot leave stale valid output copyable as current. Programmatic synchronization must not create feedback loops or extra History entries.
- Keep color parsing/formatting and sRGB correctness in the Utility core. A reusable channel control in sofui owns interaction and presentation only.

### Required correction R5: confirm meaningful generator replacement

- Identifier Generator and Sample Data must consider the entire meaningful current workspace state when deciding whether restore replaces a different nonempty session. Comparing only generation configuration is insufficient when two batches use identical settings but contain different results.
- Warn before replacing a different generated batch or meaningful edited configuration. A Sample Data schema being edited does not count as empty merely because it has no current valid result.
- Do not warn for an empty workspace or an already-equivalent snapshot. Cancelling confirmation preserves the current state; confirming applies exact captured controls and results.
- Restore never calls the generator, reads a new clock/random source, or records a new Utility Operation. Pending obsolete computations must not overwrite the restored state.

### Required correction R6: saved Random String controls

- Persist the latest length, count, uppercase/lowercase/digit/symbol choices, ambiguous-character exclusion and custom alphabet independently of Utility History. Changes to controls must survive relaunch without requiring the owner to enable History or generate output first.
- Restore valid saved controls on opening. Fresh defaults remain length 20, count one, upper/lowercase, digits and safe symbols enabled, with ambiguous-character exclusion enabled. Preserve the supported length range 1–4,096 and count range 1–100.
- Store control configuration only. Generated strings, History entry data and generation sequence bookkeeping do not belong in ordinary preferences. Loading saved controls must not generate output or record History.
- Use the profile's own preference namespace, with versioned/validated loading and deliberate handling of missing or malformed configuration. Persistence failures must not be falsely presented as successful saves or break the visible Utility result.

### Preserved product and Utility contract

- The product remains private, local-only, offline-capable and Apple-Silicon macOS-focused. No accounts, telemetry, synchronization, hosted Utility execution or Clipboard monitoring are introduced.
- Preserve Library, Recent and Favorites scopes; grouped searchable catalog; keyboard navigation; source-defined Registry metadata; retained per-Utility in-memory sessions; separate Settings; and trailing collapsible History.
- Preserve Control-Option-Space as the configurable default Launcher shortcut. Registration failure retains the last working shortcut with an inline diagnostic. The Launcher is transient and nonactivating, appears on the active display/current Space including full-screen contexts, and dismisses by Escape, click-away or repeated shortcut without stealing the previous application's activation. Choosing a Utility opens and activates the Workbench.
- Closing the Workbench keeps the application running. Dock reopen restores the window and current workspace. Explicit Quit ends the process. No login item or persistent menu-bar feature is added.
- Preserve explicit Paste/Copy, useful copy feedback, Unicode-safe editing/selection/deletion/undo, task-appropriate debounce, explicit generation/expensive actions, and neutral empty states. Invalid input and obsolete computations never masquerade as current successful output.
- History keeps the newest 25 entries per Utility using versioned per-Utility snapshots, stable entry identities and timestamps, atomic replacement and serialized mutations. Recording defaults on globally and per Utility, except JWT defaults off. Preview and restore do not execute or record; deliberate repeated generations remain separate operations.
- Preserve one-second pointer hold for Clear Utility and two seconds for Clear All, cancellation on early release/exit/Escape, and ordinary confirmation for keyboard activation. History payloads are never logged, indexed, synchronized or exported. Do not add persistent workspace autosave or separate encryption/Keychain machinery.

| Utility | Required retained behavior |
| --- | --- |
| JSON | Format, minify, validate, configurable indentation/key sorting, JSON Pointer and simple dot/bracket queries; preserve numeric meaning and array order; invalid JSON is diagnosed without silent repair. |
| YAML/JSON | Single YAML 1.2 Core-oriented document; anchors/aliases and string-keyed mappings; reject duplicates, unsupported tags, multiple documents and values incompatible with JSON fidelity; disclose loss of comments, formatting and alias identity. |
| Base64 | UTF-8 encode/decode, standard and URL-safe alphabets, padding control and strict invalid-input diagnostics. |
| URL Encoding | Distinct Path Segment and Query Value modes; strict UTF-8 percent decoding; uppercase percent bytes; spaces encoded as percent-20 and literal plus preserved on decode. |
| Hashes | Explicit SHA-256/384/512, SHA-1 and MD5 over exact UTF-8; hex casing/Base64; visible legacy labels; explicit hashing of empty bytes is valid. |
| Identifier Generator | UUID v1/v3/v4/v5/v6/v7 with validation, normalization and relevant controls; ULID random/process-local monotonic modes; KSUID random and explicit ordered batches capped at 65,536; inspection and exact batch restore. |
| Timestamps | Automatic/manual Unix seconds or milliseconds, ISO 8601 and named-zone local time; ambiguous inference and repeated/nonexistent DST wall times diagnosed; restoration uses the captured instant. Automatic signed integer input of at most ten digits means seconds, exactly thirteen means milliseconds, eleven/twelve requires explicit selection; fractional seconds require seconds mode and ISO Auto requires offset or Z. |
| JWT Decoder | Separate readable header/payload and opaque signature, segment-specific diagnostics, no signature/key/trust/claim-validation assertions, History off by default. |
| Regex | Rust regex dialect, supported flags, all matches/captures and replacement preview, clear unsupported look-around/backreference diagnostics, engine and replacement-syntax guidance, with the corrected scheduling and bounds above. |
| Case Conversion | Existing nine styles with one visible deterministic segmentation policy and Unicode/grapheme-safe, locale-independent output. |
| Whitespace Conversion | Existing nine actions, LF default, tab width 1–8 with default 4, tab-stop expansion, leading-indentation-only spaces-to-tabs, explicit line-ending and dedent behavior. |
| Color Conversion | Bounded sRGB HEX/RGB(A)/HSL(A), deterministic alpha/rounding, synchronized selection and invalid-text handling, without adding color spaces. |
| Sample Data | Typed reorderable fictional fields, JSON/CSV, default ten rows, maximum 1,000 rows and 50 fields, schema validation, correct quoting and exact schema/output restore, without external data. |
| Random String | System cryptographic randomness, unbiased alphabet sampling, supported controls/exclusions and saved controls, honest entropy, per-item Copy and Copy All, exact restoration without generation. |
| Text Diff | Split/unified modes, exact old/new text snapshots, selection/copy/scrolling/resize, renderer loading/failure/recovery states, UTF-8 correctness and the existing disclosed whole-line fallback for complex emoji. |

- Each Utility remains fully registered and usable, with diagnostics, Clipboard behavior, session continuity, applicable recording, preview and restore. Cleanup must not turn implemented Utilities into placeholders or silently change their stable snapshot/Registry identities.
- Preserve independent vectors and relevant regression cases before deleting their old source location. The purpose is contract continuity; test expectations must not be recreated by copying the implementation algorithm under test.

### Native redesign and owner checkpoint

- The chosen direction is a compact, polished native Workbench: clear typography, restrained borders, efficient spacing and controls, deliberate hierarchy and readable content. Retain the agreed navigation rather than adding dashboard or navigation concepts.
- Use the same reusable sofui controls for the preview and its representative interactions. The preview is a real GPUI window, not an HTML image/mockup or a separate web application. It can use synthetic in-memory content to avoid product-data mutations.
- Show the shell with realistic catalog density, central text input/result, trailing History, representative selection/numeric/color/confirmation controls, and both theme presets. Cover sufficient states to assess focus, disabled/invalid feedback, resizing and density. This is one design direction and one preview review, not a required multi-variant design contest.
- Present the runnable native preview and actual visual evidence to the owner. Wait for review before applying the visual design across all Utilities. Resolve requested design feedback within the agreed direction, then record the accepted decision.
- After approval, apply the accepted treatment consistently to the Workbench, Utility workspaces, Launcher, Settings and gallery while preserving behavior and session state. Temporary preview scaffolding must not become an unexplained second shipped application or production-only switch.
- Retain the established Release icon artwork and its orange DEV treatment for Debug. Move reusable resources out of Swift-specific packaging; the native packaging process must no longer require the old Swift icon generator.

### Product identity, versions and data isolation

- Release display name: **SofDevTool**. Release bundle identifier and preference domain: **com.gerardocalia.sofdevtool**.
- Debug display name: **SofDevTool Debug**. Debug bundle identifier and preference domain: **com.gerardocalia.sofdevtool.debug**.
- Give each profile its own Application Support namespace keyed by its bundle identity, separate output bundle and appropriate icon. Both can exist together without overwriting each other's bundle, preferences, History or saved controls. Development identity remains visible inside the application as well as in the Dock.
- Application version is **0.2.0**; sofui version is **0.1.0**. Derive application version metadata from one authoritative package value. Include a deliberate bundle build identifier, immutable Git revision and build channel in inspectable bundle/application information. A revision alone is not a semantic version, and hardcoded duplicate version strings are not the intended release mechanism.
- Release verification records the exact clean source revision used to produce the tested artifact. If development builds include uncommitted changes, identify that condition honestly rather than presenting the revision as the complete contents of the build.
- Preserve an explicit test-data-root override for automated/native checks. Debug/Release tests must not accidentally resolve to the owner's ordinary data locations or change another profile's global settings.
- No import, migration or deletion of Swift data is included. No migration layer for the prior Rust namespace is needed for this owner: the metadata-only check found that namespace absent. New identities must not adopt the existing legacy Swift data directory by coincidence.

### Text Diff assets and provenance

- Retain the embedded WebView architecture behind an application-owned renderer interface. Its resources must remain bundled locally, independent of process working directory and the source checkout.
- Preserve the editable JavaScript entry source, exact npm lockfile, bundle-generation recipe, upstream provenance and required license notices before deleting the Swift wrapper and duplicate vendor copies. Maintenance must be possible without extracting source from the minified bundle.
- Adapt asset verification and notice generation to the retained runtime. Inventory actual bundled dependencies and applicable attribution; remove misleading unused Swift/Yams and optional-bundle inventory claims without removing notices still required by retained code.
- Preserve readiness idempotence, stale request rejection, failure/recovery feedback, UTF-8 transport, complex-emoji behavior, clipping, scaling, focus handoff and native copy. Existing string-based bundle adaptations must remain explicitly verified or be moved into an owned source bridge with equivalent behavior.
- Preserve local-resource and navigation restrictions. No remote script/font fetches or payload-bearing network traffic may be introduced. Asset compilation may require development dependencies; the installed application must not need Node/npm or network access to run.
- Rebuilding from the exact lockfile and verifying the resulting resources/provenance must be documented. A Rust compile accepting an already-generated bundle is not proof that the asset pipeline remains maintainable after cleanup.

### Makefile, packaging and installation

- Provide one root Makefile as the discoverable command entrypoint. Recipes delegate to a shared set of authoritative scripts where useful; checks and packaging rules must not diverge across aliases.
- The command contract includes help, run, gallery, format, default verification, full verification, Release packaging and installation. Use the command names **make help**, **make run**, **make gallery**, **make format**, **make verify**, **make verify-full**, **make release**, and **make install** in the README.
- **make run** builds and launches the correctly identified Debug application bundle. **make gallery** opens the independent sofui gallery. **make format** applies Rust formatting. **make verify** checks formatting, first-party Clippy warnings, meaningful tests and Debug/application/gallery builds. **make verify-full** adds Release and applicable asset/packaging/library-isolation checks; focused native verification remains explicitly documented and honestly reported.
- **make release** builds the Release bundle with version/revision/channel metadata, established icon and correct notices. Debug and Release packaging share maintained implementation where possible, honor an overridden Cargo target directory and produce different artifacts. A packaging failure must propagate a nonzero result rather than leave a success message for an incomplete bundle.
- **make install** builds or verifies the intended Release artifact and installs it into the owner's home Applications directory by default. Expose a documented destination override. Scope replacement to the intended application bundle and preserve existing data. Installation must not select whichever profile happened to be built last or silently launch Debug.
- Provide a locally launchable macOS bundle without requiring paid distribution infrastructure. Keep the macOS 14 deployment baseline and record any concrete dependency conflict rather than silently increasing the minimum OS. Public distribution, Developer ID provisioning, notarization and App Store work are excluded.
- Final implementation acceptance includes actually running the default installation path and launching the installed Release outside the checkout with isolated test data. Verify fresh state, a successful recorded operation, relaunch persistence, expected version/revision/channel and correct resource resolution. This is separate from writing the present specification.

### Delivery sequence and completion criteria

1. Establish current contracts and regression checks, correct the six defects, and prepare the application/sofui responsibility split. Keep verification meaningful as shared interfaces change.
2. Build the compact native preview using representative real library controls and synthetic data. Present it for owner review. Full visual rollout waits here for approval.
3. Apply the approved design throughout the application, complete sofui interfaces, gallery and independent-consumer evidence, and remove temporary product scaffolding.
4. Finish root-workspace relocation, Swift retirement, renderer/icon resource ownership, current documentation, version/profile identities and Makefile packaging/install workflow. Preserve necessary behavioral/provenance evidence before deleting old material. Independent preparation may happen earlier if it does not bypass the preview gate.
5. Run appropriate regression, integration, component, asset, build and native gates against the resulting product. Perform actual Release installation/launch acceptance with isolated data and record limitations accurately.
6. Review the complete resulting diff for standards and specification compliance, resolve findings, and leave a clean, committed local branch suitable for merging into the target branch. Use GitButler for repository writes. No remote push, pull request or merge is authorized by this delivery scope.

Completion requires all six corrections verified; all fifteen Utilities operational; accepted redesign applied; component-only sofui proven independently consumable; no executable Swift product/build dependencies or obsolete active guidance; working documented Makefile commands; distinct Debug/Release identity and storage; correct installed Release metadata and behavior; accurate evidence; and a clean local branch. A green existing domain test suite, a running preview or a newly written spec alone is not completion of this implementation.

## Testing Decisions

### Agreed seams and test quality

- The owner has already confirmed the scope containing these test seams. This specification synthesizes that agreement; it does not start another interview or require a second approval of unchanged testing choices.
- Prefer the highest useful existing seam: Utility request/result/snapshot contracts for domain behavior; application session plus temporary storage for coordination; sofui's public interactions and an independent consumer for library behavior; and the actual packaged macOS app for OS/native integration. These are distinct externally meaningful responsibilities, not a requirement to add a new test interface for every helper.
- Preserve and extend the existing contract tests, temporary-directory History/preferences tests, controlled session-revision tests, deterministic clock/random sources and independent fixtures. Use GPUI's available public test support for input, keyboard, pointer and redraw behavior where it reaches the real interaction.
- Test observable outcomes, events and persisted state. Avoid asserting private component nesting, every style value, implementation method names, or expected results calculated by duplicating the production algorithm.
- Add regression tests at the seam that reproduces the defect before fixing it where practical, and verify that the failing case becomes green. Pure helper tests alone do not cover a wiring defect in a workspace or a cross-window History action.
- Use isolated data roots, fake clocks/randomness and controlled completion order. Do not inspect, log or copy the owner's History payloads to establish a test result.

### Required regression and integration matrix

| Area | Required observable evidence |
| --- | --- |
| Regex execution | A controllably slow evaluation does not block UI/application work; newer input, clear and restore obsolete it; the winning result alone publishes/records. Rapid changes keep running/pending work bounded. A demanding real-engine case exercises actual execution, not only a mocked timer. |
| Regex limits | Boundary/over-limit pattern, program, text, replacement, match/capture and output cases preserve input, show a refusal and never record partial success. Engine wording accurately describes available guarantees. |
| History deletion | Populate multiple open Utility workspaces, select entries, then clear one/all from Settings. Visible and hidden inspectors, selection and pending restore reconcile immediately, while current workspace results stay intact. Cover failure and unknown-Utility-file behavior. |
| History failure/recovery | Create a deterministic write failure, assert the previous file/result survive, and prove subsequent ordinary operations do not write while paused. Failed recovery stays paused; successful recovery clears the state without duplicate entries. Corrupt files remain untouched and other Utilities keep working. |
| History policy | Global and per-Utility toggles preserve each other and retained entries; JWT defaults off; newest-25 retention, ordering, one-shot recording, deliberate repeats and exact no-execution restore remain correct. |
| Color interaction | Exercise the real control/event path. Adjust a channel, then adjust again before and after settlement; text, channels, preview, Copy and settled History agree. Test invalid input, clamping and absence of feedback-loop duplicate operations. |
| Generator restore | Generate two different outputs with identical controls; restoring the first requests confirmation. Cancel preserves the second; confirm restores the first exactly without randomness/clock calls or a new entry. Equivalent/empty sessions avoid needless warnings; edited Sample Data schemas are protected. |
| Random String preferences | Change each supported control, reconstruct/relaunch under the same isolated profile with History disabled, and observe the saved configuration without output generation. Test fresh defaults, invalid stored settings, write failure and Debug/Release separation. Ordinary preferences contain controls only. |
| Component identity/focus | Repeated visible labels remain safe; stable identity and focus survive redraw and label changes; keyboard and pointer activation agree; disabled controls do not invoke actions. |
| Text interfaces | Silent assignment, user editing, change notifications and undo have the documented distinction. Preserve multiline Unicode editing, grapheme deletion, selection, read-only output and undo/redo regressions. |
| Hold and confirmation | Completion occurs only after the intended duration; early release, exit and Escape cancel; progress resets correctly; keyboard activation uses confirmation. Tests exercise reusable component behavior and the app's destructive action separately at their public seams. |
| Lists and theme | Keyboard/pointer selection and unavailable/empty states work with generic supplied data. Theme changes update multiple open surfaces and editor styling without resetting sessions, recording operations or changing Recents. Custom tokens work through the public interface. |
| Library independence | Build/run the gallery without app/core dependencies; compile an independent consumer using only sofui's documented setup and declared dependencies. No product paths, resources, persistence or lifecycle behavior are required. |
| Complete catalog | Preserve all fifteen request/result/snapshot contracts, independent vectors, resource boundaries, diagnostics, Clipboard actions and workspace continuity under the redesign. |
| Renderer | Rebuild/verify retained assets and notices; exercise local loading, Unicode, stale callback rejection, readiness, failure/recovery, split/unified views, resizing/clipping, scrolling, selection/copy and focus handoff. |
| Profiles/install | Inspect both bundle identities/icons/metadata, exercise isolated profile preferences/History, honor a nondefault Cargo target and install destination, and launch installed Release outside the checkout. Relaunch preserves its test operation and does not touch legacy data. |
| Repository/docs | Confirm Cargo and Makefile work from the root; executable Swift/Xcode requirements and stale active instructions are gone; retained documentation links resolve; current commands, dependency provenance and local-delivery scope match the actual project. |

### Native verification and evidence limits

- Use actual GPUI/native execution for the preview and final interaction claims. Screenshots document appearance; they do not alone prove control behavior, storage isolation, correct app identity or successful installation.
- Exercise the global shortcut from another app, nonactivating Launcher presentation/dismissal, full-screen/current-Space behavior, keyboard search/open, Settings separation, Dock reopen, explicit Quit, editor focus and WebView interaction. Keep these checks isolated and record what actually ran.
- Record host macOS/architecture, toolchain, source revision, commands, outcomes and artifact/evidence locations. The reviewed baseline passed 257 tests and Debug/Release builds, but that result predates these changes and does not close the six new regressions.
- Keep the macOS 14 deployment target while distinguishing it from runtime testing. The existing native evidence is on macOS 26.2 arm64; macOS 14/15 runtime, cross-display/DPI transitions and any other unexercised scenario remain explicitly unverified unless actually tested.
- IME composition remains the owner's previously accepted nonblocking evidence exception. Do not fabricate a passing check or require changing system input sources to restart a settled blocker. Record any newly obtained evidence accurately. Full VoiceOver certification is outside scope; ordinary keyboard/focus/label usability remains required.
- Offline acceptance covers resource containment and a packaged run with network access denied where practical without changing the owner's system network configuration. If evidence is structural only, label it structural rather than claiming a network-denied run or packet capture.
- For this documentation-only publication, validate the specification structure, tracker links, accepted decisions and absence of product changes. Do not run application builds or native tests and present them as implementation of this spec.

## Out of Scope

- Product implementation, creation of the native preview, installation, or source cleanup during the present spec-writing request.
- Maintaining a runnable Swift fallback, preserving a second product in the working tree, or retaining superseded executable prototypes as active project material.
- Reading, importing, migrating or deleting legacy Swift user data; silently uninstalling a separately installed old app; adding an unnecessary migration layer for the absent previous Rust namespace.
- Building a speculative general-purpose component catalog, universal application/schema framework, service locator, dynamic plugin system or multiple Utility execution backends.
- Reimplementing text-editing fundamentals or adopting a full external visual system as sofui's public API.
- Independent per-window themes, additional required theme presets, new navigation concepts or a redesign outside the agreed compact native Workbench direction.
- Creating or publishing a separate sofui repository/package in this increment. The library must be ready for later separate release, with independent consumption proven locally.
- New Utility families, file/batch workflows, persistent workspace autosave, ICU emulation, expanded color spaces, or a native replacement for the retained web-backed Text Diff.
- Windows, Linux or Intel delivery; full VoiceOver certification; unsupported runtime claims for untested macOS versions.
- Accounts, telemetry, synchronization, hosted Utility services, external data sources, Clipboard monitoring, public distribution, notarization or App Store delivery.
- Git history rewriting solely to erase the existence of Swift, remote publication, a pull request, or merging the implementation branch into the target branch.
- Numeric performance improvements, hard Regex execution deadlines or complete correctness claims unsupported by measurement and the actual engine guarantees.

## Further Notes

### Confirmed owner decisions

| Decision | Agreed outcome |
| --- | --- |
| Repository | Root Cargo workspace with application, core and UI crates; Rust application only. |
| Library | sofui; component logic only; existing controls plus demonstrated application needs; independent release later. |
| Visual direction | Compact, polished native Workbench; established navigation retained; one native preview review before full rollout. |
| Themes | Application-wide observable selection and custom tokens; Graphite and Catppuccin Frappé presets retained. |
| Release identity | SofDevTool; com.gerardocalia.sofdevtool. |
| Debug identity | SofDevTool Debug; com.gerardocalia.sofdevtool.debug; separate app, data and icon treatment. |
| Versions | Application 0.2.0; sofui 0.1.0; app also identifies its source revision and build channel. |
| Artwork | Established application artwork and orange DEV treatment retained. |
| Installation | Release to ~/Applications by default, with destination override; actual install and isolated-data launch included in future implementation acceptance. |
| Data roots | Separate Application Support namespaces named com.gerardocalia.sofdevtool and com.gerardocalia.sofdevtool.debug; no legacy Swift data changes. |
| Documentation | Current contracts and useful evidence; superseded plans/spikes/WIP removed from the working tree, recoverable through Git. |
| Delivery | Clean, committed, merge-ready local branch; no push or PR. |
| Present task | Write and locally publish this specification only; implementation has not started. |

### Baseline and operational observations

- The review compared the current port with the pre-port base and approved migration contracts. At the start of this spec-writing task, all 419 tracked files were byte-identical to the current workspace commit; no product-code edit from the interrupted implementation turn had occurred.
- The observed migration branch is feat/rust-gpui-migration at fd7a08e; the checkout uses GitButler's synthetic workspace commit d3ca880. These identifiers describe the observed baseline, not a requirement to hardcode future tooling around them.
- Raw index deletion/untracked pairs were not actual missing source: GitButler reported no uncommitted changes when permitted to access its local database. Future implementation must preserve that distinction and use GitButler for repository writes rather than manually repairing the index with raw Git commands.
- No matching installed application was found in the two standard Applications locations during the bounded metadata check. The previous Rust Application Support namespace was absent; the legacy Swift namespace existed. No personal preference or History payload was inspected. These are dated observations, not permission to delete data or assumptions that must remain true on another machine.
- The retained Text Diff bundle matches the maintained vendored bundle, whose source/build recipe currently lives with the Swift wrapper. Its provenance includes PierreDiffsSwift 1.2.4 at revision c2249d7890de957a96480711152d90a06fa1222b. Preserving that non-Swift source/provenance is a prerequisite to retiring the wrapper tree safely.
- Useful review probes showed Regex execution occupying about 1.67 seconds for a 256 KiB input in a Release-mode core probe, and ordinary History writes succeeding while its paused flag remained true. These are diagnostic evidence of the original defects, not final acceptance measurements.
- The two remaining owner checkpoints have different meanings: the scope has been agreed, while the future native visual preview still needs review. The later instruction to produce a spec explicitly prevents treating scope confirmation as permission to implement in the present task.

### Sources and tracker publication

- [Domain glossary](../../CONTEXT.md).
- [Retained contract-vector audit](evidence/fixture-preservation.md) and [public core vectors](../../crates/core/tests/retained_contract_vectors.rs), which preserve distinct applicable expectations from the preceding implementation. Its superseded planning material is recoverable from Git history.
- [Rust-only planning index](README.md) and [proposed ticket breakdown](ticket-proposal.md).
- [Prior migration evidence](../../docs/evidence/previous-migration/README.md), retained as dated observations rather than current acceptance.
- [Local issue-tracker convention](../../docs/agents/issue-tracker.md).
- [Previously recorded release evidence](../../docs/evidence/ticket-22.md), which describes the prior candidate and its limitations.

This document is the canonical complete specification for the separate Rust-only completion initiative. It retains its **ready-for-agent** triage status; the derived ticket breakdown requires owner approval before individual tickets are published. Moving the specification does not change its agreed product scope or authorize implementation, preview creation, verification of changed code, commits, installation or release.
