# SofDevTool product and architecture handoff

This is the implementation contract for the initial SofDevTool scaffold and the architectural baseline for the complete first release. Read [the domain glossary](../../CONTEXT.md) before changing domain language. Follow the linked decision ticket when this handoff omits rationale or a Utility-specific edge case.

## Product brief

SofDevTool is a private, local-only macOS Developer Toolbox for one owner. It replaces trips to online converters and oversized developer applications with focused native Utilities. It is not an IDE, terminal, Git client, database browser, API client, browser DevTools replacement, or customer-facing service.

The product has no accounts, cloud synchronization, telemetry, hosting, paid APIs, or required Apple Developer Program membership. Core Utilities work offline. Preferences and configurable Utility History stay on-device. The application targets Apple Silicon with a macOS 14 deployment target, covering macOS 14, 15, and 26; each version's runtime support must be evidenced separately.

The complete first-release catalog is defined in [Define first-release Utility contracts](issues/02-define-first-release-utility-contracts.md). It contains:

- JSON;
- YAML/JSON conversion;
- Base64 and URL encoding;
- Hashes;
- Identifier Generator;
- Timestamps;
- JWT Decoder;
- Regex;
- Text Diff;
- Case Conversion;
- Whitespace Conversion;
- Color Conversion;
- Sample Data; and
- Random String.

The post-scaffold implementation details for the remaining ten Utilities are defined in the [remaining first-release Utilities implementation specification](remaining-utilities-implementation-spec.md). That addendum owns their refined modes, resource policies, dependency decisions, and completion criteria; this handoff continues to own the shared architecture.

The initial scaffold is a representative vertical slice, not the complete catalog. It must ship working JSON, Base64, Random String, and the complete Identifier Generator with UUID, ULID, and KSUID. An empty or disabled placeholder does not count as an implemented Utility.

## Scaffold completion contract

The scaffold is complete when all of the following are true:

- one checked-in Xcode project and shared scheme build a native Swift 6 application with a macOS 14 deployment target;
- the project has exactly three targets: the application, one Swift Testing unit/integration bundle, and one XCTest UI bundle;
- the Workbench shell implements Library, Recent, and Favorites scopes, catalog search, keyboard selection, persistent Utility workspaces, a collapsible History inspector, a separate Settings destination, and the Utility Launcher;
- JSON, Base64, Identifier Generator, and Random String satisfy the behaviors in this handoff through real domain implementations;
- explicit Clipboard, History, global-shortcut, and required AppKit integration seams are wired to production adapters;
- History recording, persistence, preview, restore, controls, deletion, and isolated failure behavior are operational;
- `scripts/format`, `scripts/verify`, and `scripts/verify --full` implement the agreed local gates;
- the required automated scenario matrix passes;
- the full gate passes on the current macOS 26 host and the manual release checklist is recorded there; and
- build/test evidence distinguishes verified behavior from pending macOS 14 and 15 runtime checks.

The implementation may choose private type names and small presentation details that do not alter the contracts below. A product ambiguity or conflict returns to its owning decision ticket; it is not silently decided in code or tests.

## Technology baseline

Use Swift 6 and SwiftUI, with narrow AppKit adapters only where SwiftUI does not provide the required macOS behavior. Use Swift Testing for domain, integration, persistence, and adapter tests; use XCTest/XCUIAutomation for the focused UI suite. [Choose the macOS application technology stack](issues/01-choose-macos-technology-stack.md) owns the stack decision.

Use Apple frameworks by default. The scaffold may use `KeyboardShortcuts` behind an application-owned global-shortcut protocol. Pin every accepted Swift Package Manager dependency in `Package.resolved`. Do not add a workspace, project generator, local Swift packages, SwiftData, SwiftLint, runtime plugin loading, or hosted CI to the scaffold.

Before adding any other dependency, record its maintenance, license, offline/privacy behavior, binary impact, accessibility implications, and replacement seam. [Validate PierreDiffsSwift for Text Diff](issues/10-validate-pierre-diff-integration.md) is the accepted review for the later Text Diff renderer; it does not require adding Pierre to the representative scaffold.

## Source and target shape

Keep one application target with feature-oriented folders that act as codebase modules through small interfaces rather than extra build targets. Use this conceptual source map; exact private filenames may follow the implementation:

```text
SofDevTool/
  Application/          app entry, lifecycle, dependency composition
  Shell/                Workbench, Launcher, Settings, catalog state
  Registry/             Utility Definition, registry, search
  History/              entries, repository, inspector, recording policy
  Platform/             clipboard, shortcut, window/activation adapters
  Presentation/         genuinely shared SwiftUI presentation components
  Utilities/
    JSON/
    Base64/
    IdentifierGenerator/
    RandomString/
SofDevToolTests/         mirrors domain and integration ownership
SofDevToolUITests/       focused end-to-end shell and interaction flows
scripts/
```

Add a concise root `AGENTS.md` during scaffold construction. It points agents to `CONTEXT.md` and this handoff, maps the actual source tree, names the authoritative commands, and records module/seam, privacy, test-placement, and change-checklist rules. Owner setup belongs in `README.md`; decision rationale remains in its ticket.

## Module boundaries

[Design the Utility module interface](issues/05-design-utility-module-interface.md) owns these boundaries.

### Shell

The shell owns the Utility Registry, selected Utility, Library/Recent/Favorites scopes, catalog search, navigation, Settings, History presentation, and application-level persistence. Opening a Utility updates shell-owned Recents even when no Utility Operation completes. Favorites and Recents store immutable Utility IDs.

The process retains one in-memory Utility Workspace Session for every opened Utility. Switching Utilities replaces only the visible workspace and preserves each session's controls, input, diagnostics, and result. Cross-launch input/result restoration occurs only through enabled Utility History.

### Utility Registry

The registry is a small, explicit, compile-time catalog. Every Utility Definition contains only:

- a short immutable Utility ID;
- display name;
- exactly one primary category;
- search aliases;
- catalog symbol;
- History capability/default; and
- a type-erased SwiftUI workspace factory.

The fixed categories are **Format & Convert**, **Encode & Inspect**, **Generate**, and **Compare & Test**. Alternate discovery terms are aliases. Only the heterogeneous workspace factory is type-erased; concrete Utility types remain strongly typed.

### Utility modules

Each Utility owns its definition, workspace, session state, domain engine, validation, commands, focus behavior, concrete results/errors, snapshot schema, and tests. Utilities remain independent modules; there is no universal `execute(input)` protocol.

A Utility workspace context contains only explicit Clipboard access and a History recorder. Clocks, randomness, and other deterministic dependencies are injected inside only the Utilities that need them. The context is not a service locator.

The shared diagnostic envelope contains severity, a readable message, and an optional source location or range. A session owns its debounce, asynchronous work, cancellation, and input-revision checks. Only the latest current request may update visible output or offer a History snapshot.

### History boundary

A completed valid Utility Operation may offer one Utility-owned, versioned Utility Operation Snapshot. Invalid and intermediate states offer none. Each snapshot carries its immutable Utility ID and integer schema version; its Utility owns decoding and migration. The History repository treats snapshot bytes as opaque.

All first-release Utilities support History. They default on except JWT Decoder, which defaults off.

## Workbench and native behavior

[Prototype the main window and Utility Launcher](issues/03-prototype-main-window-and-launcher.md) and [Decide native integration boundaries](issues/04-decide-native-integration-boundaries.md) own the interaction contract.

The main window is a dense, dark-only native Workbench:

- a compact toolbar exposes Library, Recent, and Favorites scopes, one central Utility search field, Utility Launcher, and Settings actions;
- Library is the default scope;
- a two-column split view places the grouped, searchable, keyboard-navigable Utility list on the left and the selected Utility's persistent workspace on the right;
- category grouping does not add another navigation level;
- empty search shows a neutral no-results state with a clear-search action;
- arrow keys navigate visible results, Return opens, and focus remains sensible as selection changes; and
- Settings opens separately without replacing the selected Utility or discarding its session.

The transient Utility Launcher searches the same registry and opens a Utility in the main window; it is not a second execution surface. Its default global shortcut is Control-Option-Space and is configurable through native recording in Settings. Invalid or conflicting edits keep the last working shortcut and show an inline error.

While the app is running, the Launcher appears centered on the active display and current Space, including over a full-screen app. Escape, click-away, or repeating the shortcut dismisses it without disturbing the active application. Selection dismisses it, activates the main window through normal macOS behavior, and opens the Utility.

The application remains running after its last window closes and quits only through explicit Quit. Dock activation restores the main window's prior size/position and last selected Utility. It does not launch at login. It keeps standard menus and Dock presence but no persistent menu-bar item.

Force Dark appearance with no appearance preference while honoring accent color, Increased Contrast, Reduced Transparency, text scaling, and other native accessibility behavior. Clipboard access occurs only after explicit Copy or Paste through the narrow adapter; there is no clipboard monitoring or classification.

## Shared Utility interaction contract

Utilities are text-first. File and batch-file workflows are outside the first release. Lightweight deterministic work is live and debounced; generators and expensive work use explicit execution. Applicable workspaces provide labeled input/result regions, inline diagnostics, Copy Result, Clear, optional Swap, and task-appropriate focus without being forced into one layout.

Empty input is neutral. Invalid current input replaces the result with a precise diagnostic, never a stale result. Large input is not silently truncated or partially processed; the Utility may warn or require explicit execution. Clipboard reads/writes are explicit, and successful Copy receives an unobtrusive confirmation. Multi-result generators support per-result Copy and newline-separated Copy All.

One settled valid computation or explicit generation request is one Utility Operation and produces at most one History entry. Intermediate edits, invalid states, preview, and restoration are not Utility Operations. Controls may persist as preferences; inputs and results persist only through enabled Utility History.

## Representative Utility contracts

### JSON

Open in Format mode with two-space indentation, live validation, unsorted keys, and explicit controls for querying and key sorting. Validate with line/column diagnostics, pretty-print with configurable indentation, minify, explicitly sort object keys while preserving array order and numeric values, and query through JSON Pointer plus simple dot/bracket paths. Never repair malformed JSON silently.

### Base64

Open in Encode mode with the standard alphabet, padding enabled, and UTF-8 text. Keep Encode/Decode and Standard/URL-safe as visible controls; padding is configurable. Encode and decode UTF-8 with standard and URL-safe alphabets. Diagnose invalid input instead of guessing output.

### Identifier Generator

Expose UUID, ULID, and KSUID through a top-level format selector; reveal only the selected format's controls. Never present ULID or KSUID as UUID versions. Identifiers are collision-resistant, not guaranteed unique or secret.

UUID supports v1, v3, v4, v5, v6, and v7; omit v8. Default to one lowercase hyphenated v4 and recommend v7 for sortable UUIDs. Support single/batch generation, strict validation, normalization, casing, hyphen controls, and version-specific inputs where required.

ULID supports random and process-local monotonic modes, single/batch generation, strict validation, canonical uppercase normalization, and timestamp inspection as Unix milliseconds and ISO 8601. Describe monotonic ordering as local to one generator, never coordinated across applications or machines.

KSUID supports cryptographically random single/batch generation, strict case-sensitive validation, and timestamp inspection. Provide an explicit ordered-sequence batch mode with its 65,536-result limit; ordinary KSUID generation is not strictly monotonic.

### Random String

Generate with the system cryptographic random-number generator. Expose length, count, uppercase/lowercase, digits, safe symbols, custom character sets, and ambiguous-character exclusion. Default to length 20, one result, upper/lowercase, digits, and safe symbols enabled, and ambiguous characters excluded. Restore the latest controls on reopening. Show entropy only when it is honestly calculable and never frame the Utility as password management.

## Utility History

[Design Utility History storage and privacy](issues/06-design-history-storage-and-privacy.md) owns this contract.

History is globally enabled by default, with a retained per-Utility switch. Turning either switch off stops new recording immediately and preserves existing entries. The global switch does not rewrite saved per-Utility choices. Keep the newest 25 entries per Utility; the cap is not configurable in the scaffold.

The app-owned entry envelope contains stable entry ID, UTC capture timestamp, immutable Utility ID, snapshot schema version, and opaque snapshot bytes. Record every distinct completed Utility Operation, including deliberate identical repeats. Serialize writes independently per Utility, order by timestamp then stable ID, trim before persistence, and attempt the write immediately.

Use a Foundation/Codable repository with one versioned JSON file per Utility in Application Support and atomic replacement. UserDefaults holds only small preferences, never Utility payloads. History data is ordinary local application data: do not log, index, sync, export, encrypt separately, or put it in Keychain.

The selected Utility's collapsible inspector lists newest first. Selection asks the owning Utility to decode and render a preview; the repository stores no derived title or search excerpt. Restore explicitly applies the snapshot's controls, input, and result without rerunning. Warn only when it replaces a different non-empty session. Restore itself does not record.

Clear Utility deletes that Utility's persisted and in-memory History, preserving the current workspace session. Pointer use requires a one-second press-and-hold. Clear All deletes every History file, including absent Utilities, after a two-second hold. Show progress and cancel on early release, pointer exit, or Escape. Keyboard and assistive-technology activation use an ordinary destructive confirmation alert. Deletion is permanent and has no undo.

A write failure preserves the visible Utility result and last valid file, shows a non-modal warning, and pauses recording for that Utility until a later successful retry or relaunch. A malformed file affects only its Utility, remains removable, and is not silently overwritten. An undecodable snapshot stays listed as unavailable with its timestamp. Files for absent Utility IDs stay hidden and untouched unless Clear All runs.

## Text Diff integration boundary

When the complete catalog reaches Text Diff, import PierreDiffsSwift only inside a replaceable `TextDiffRenderer` module. The Utility's workspace and domain logic never import Pierre types. The module owns `WKWebView`, display-mode translation, readiness/errors, intraline policy, accessibility accommodations, bundled notices, and dependency-specific verification.

Use an exact upstream revision with the accepted UTF-8 bridge fix or an immutable maintained fork. Preserve the real-bridge regression for accented text and multi-scalar emoji. Use word-level intraline highlighting normally and disclosed whole-line highlighting when either input contains a multi-scalar emoji grapheme. Treat ready callbacks as idempotent, disable production WebKit developer extras, and check in the complete notice inventory derived from the exact JavaScript bundle lockfile.

## Verification and evidence

[Define the verification and agent workflow](issues/07-define-verification-and-agent-workflow.md) owns the complete verification contract.

Check in:

- `scripts/format` to apply the Xcode toolchain's `swift format` using a checked-in configuration;
- `scripts/verify` to lint formatting, perform an Apple-Silicon Debug build, and run all non-UI tests; and
- `scripts/verify --full` to add a Release build and the focused UI suite.

Treat first-party warnings as errors. Put Derived Data and result bundles in an ignored repository-local artifacts directory. Preserve failing result bundles and print the underlying command on failure. The scripts are the authoritative local and later-CI surface.

The scaffold's automated inventory covers:

- registry identity, metadata, categories, inclusion, and search;
- JSON valid/invalid input, format, minify, diagnostics, and options;
- Base64 alphabets, padding, UTF-8, invalid input, and options;
- Identifier Generator format selection, validation, normalization, options, and batch invariants without random-literal assertions;
- Random String length, count, alphabets, exclusions, and invariants through deterministic randomness;
- History retention, ordering, atomic-write failure, malformed/schema isolation, deletion, preview/restore, and no rerecord on restore; and
- rejection of stale asynchronous results and snapshots.

Use published standard vectors, hand-reviewed fixtures, and property tests as independent oracles. Cover neutral empty input, Unicode, whitespace, malformed input, configuration boundaries, repeated values, and stale/cancelled work where applicable. Use fixed clocks, deterministic random adapters, temporary directories, and predicate-based UI waits. There is no numeric coverage threshold, automatic retry, probabilistic random-quality assertion, or tight wall-clock gate.

UI automation covers Library search/keyboard selection, workspace switching/session preservation, representative valid and diagnostic states, History preview/restore and accessible deletion confirmation, and Settings persistence. Give interactive controls stable accessibility identifiers and test credible labels, keyboard order, and focus restoration.

The manual release checklist covers the real global shortcut, active-Space/full-screen Launcher placement, VoiceOver speech/order, Increase Contrast, Reduce Transparency, text scaling, and related visual behavior. The scaffold is **contract-complete and verified** only for the behaviors mapped to passing evidence; tests never prove complete correctness.

A macOS 14 deployment-target build is compatibility evidence. Initially record macOS 14 and 15 runtime verification as pending. Claim runtime support for a version only after the full gate and relevant manual checklist run on that version.

## Explicitly deferred from the scaffold

- the remaining first-release Utilities beyond the representative four;
- smart clipboard detection, recommendations, prompts, or UI;
- runtime-loaded plugins or a marketplace;
- public distribution, Mac App Store work, notarization, or polished colleague distribution;
- accounts, synchronization, telemetry, networking, hosted services, or paid APIs;
- login items, Services, Share extensions, URL schemes, notifications, Touch Bar support, or a menu-bar status item;
- Intel support; and
- hosted CI or automated dependency-update infrastructure before a Git host exists.

## Implementation sequence

1. Create the project, three targets, shared scheme, feature folders, format configuration, verification scripts, ignored artifacts area, `AGENTS.md`, and owner `README.md`. The step is complete when the empty shell passes the default gate.
2. Establish application composition, Utility Registry, shell state, the Workbench, session retention, Settings, and test adapters. The step is complete when registry/search/navigation/session contract tests and focused shell UI flows pass.
3. Add Clipboard and global-shortcut adapters plus window, activation, lifecycle, Launcher, dark appearance, and accessibility behavior. The step is complete when automated adapter/shell evidence passes and manual-only cases are listed explicitly.
4. Build the History repository, recorder, inspector, Settings controls, restore/deletion flows, and failure isolation. The step is complete when every scaffold History scenario passes.
5. Implement JSON, Base64, Identifier Generator, and Random String one vertical slice at a time, including snapshots and contract-readable tests. A Utility step is complete only when its full scaffold contract maps to passing tests.
6. Run `scripts/verify --full`, exercise the manual checklist on macOS 26, and record commands, host/toolchain versions, results, and macOS 14/15 pending status. The scaffold ticket is complete only when the evidence supports every completion claim.
