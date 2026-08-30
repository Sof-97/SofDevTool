Type: grilling
Status: resolved
Assignee: Codex
Blocked by: 01, 05, 06

## Question

What repository structure and verification contract will keep SofDevTool safe and easy for coding agents to extend? Decide build commands, formatting and linting, unit and UI test boundaries, fixtures for representative Utilities, accessibility checks, supported-macOS verification, dependency policy, and the minimum continuous-integration setup that remains zero-cost.

## Comments

### Design tree — round 1, awaiting owner answers (2026-08-28)

Local inspection found a planning-only directory rather than a Git worktree or scaffold: there is no Xcode project, source tree, build script, style configuration, or CI workflow. The available toolchain is Xcode 26.6, Swift 6.3.3, the macOS 26.5 SDK, and bundled `swift-format` 6.3.0 on Apple Silicon macOS 26.2. Standalone SwiftLint/SwiftFormat, project generators, and macOS 14/15 runtime environments are absent.

The current frontier covers seven independent choices: project topology, the authoritative verification entry point, formatting/lint policy, test and fixture structure, accessibility verification, supported-macOS evidence, and dependency/CI policy.

Recommendations presented to the owner:

1. Check in one ordinary Xcode project and shared scheme, using folder-synchronized groups that mirror a feature-oriented source tree. Keep the application shell, History, platform adapters, shared presentation modules, and each Utility in explicit directories with corresponding unit/integration test directories; keep the small XCUI test target separate. Do not add Tuist, XcodeGen, a workspace, or local Swift packages until the project has a real need for them.
2. Make a checked-in `scripts/verify` command the authoritative agent/local gate. It should run style checks, an Apple-Silicon Debug build, and the non-UI test suite with deterministic paths and readable failure output; explicit flags should add UI tests or the full release gate. Document the underlying `xcodebuild` commands, but keep agents and CI on the same wrapper rather than duplicating command recipes.
3. Use Xcode's bundled, version-paired `swift format` with a checked-in configuration and a lint-only verification step; provide a separate explicit formatting command. Treat compiler warnings as errors in first-party targets. Do not add SwiftLint unless concrete rules missing from compiler and `swift-format` justify another dependency.
4. Use Swift Testing for domain, registry-contract, adapter, persistence, concurrency/revision, and snapshot-schema tests; use XCTest/XCUIAutomation only for a small set of end-to-end shell and interaction-heavy flows. Keep fixtures local to their owning Utility by default; create shared fixtures only for true cross-module contracts. Prefer readable input/expected-output fixtures and targeted visual assertions over broad golden screenshots.
5. Give interactive controls stable accessibility identifiers and automate keyboard order, focus restoration, labels, and the non-pointer destructive-confirmation path where XCTest can do so. Keep a short manual VoiceOver/display-settings checklist for behavior automation cannot credibly prove, including Increase Contrast, Reduce Transparency, text scaling, and the global Launcher over Spaces/full-screen apps.
6. Require every change to build with `MACOSX_DEPLOYMENT_TARGET=14.0`, while describing runtime evidence honestly: the current host proves macOS 26 behavior only. Run the same release checklist on macOS 14 and 15 when those physical or virtual hosts are available; do not call SDK compilation a runtime compatibility test or make unavailable older hosts block ordinary local development.
7. Keep Apple frameworks as the default, pin accepted Swift Package Manager dependencies in `Package.resolved`, and require a short rationale covering maintenance, license, privacy/offline behavior, binary impact, accessibility, and replacement seam before adding one. Keep local `scripts/verify` authoritative. Since no Git host exists yet, defer a hosted CI workflow until a repository is created; then add the thinnest provider workflow that invokes the same script rather than introducing CI-only behavior.

### Owner decision — round 1 (2026-08-28)

The owner accepted recommendations 1–7 without exceptions.

### Design tree — round 2, awaiting owner answers (2026-08-28)

The next frontier makes the accepted verification contract executable without adding speculative infrastructure: target layout, verification modes and artifacts, the representative scenario matrix, coverage/flakiness policy, agent-facing guidance, and supported-macOS release evidence.

Recommendations presented to the owner:

8. Start with exactly three Xcode targets: the macOS application, one Swift Testing unit/integration test bundle, and one XCTest UI test bundle. Inside the app target, mirror feature-oriented directories for the application shell, History, platform adapters, shared presentation code, and each Utility. Treat these as codebase modules with explicit interfaces, but do not multiply build targets merely to enforce directory boundaries.
9. Provide `scripts/format`, `scripts/verify`, and `scripts/verify --full`. The default gate checks formatting, performs a clean-enough Debug build, and runs all non-UI tests. The full gate additionally performs a Release build and the focused UI suite. Put Derived Data and result bundles under an ignored repository-local artifacts directory, preserve failing result bundles for inspection, and print the exact underlying command on failure.
10. Make the scaffold's required automated scenario matrix explicit. It includes registry metadata/search contracts; JSON valid/invalid/format/minify behavior; Base64 standard and URL-safe alphabets, padding, UTF-8, and invalid input; Identifier Generator format/normalization/batch behavior without asserting random literal values; Random String alphabet/length/count behavior through an injected deterministic random source; History retention, atomic-write failure, corruption/schema isolation, deletion, preview/restore, and no rerecord-on-restore; plus stale asynchronous result/snapshot rejection.
11. Keep UI automation focused on Library search and keyboard selection, workspace switching/state preservation, representative valid/error Utility flows, History preview/restore and accessible deletion confirmation, and Settings persistence. Do not automate the real system-wide shortcut, Spaces/full-screen placement, VoiceOver speech, or visual display-setting behavior; those belong to the manual release checklist because automation would be brittle or misleading.
12. Set no numeric code-coverage threshold and allow no automatic test retries. A behavior named by a resolved contract needs a focused test; coverage reports are diagnostic only. Tests use fixed clocks, deterministic random adapters, isolated temporary directories, and predicate-based UI waits. A flaky test is fixed or explicitly removed from the gate with a tracked reason—it is not hidden by retries.
13. Add a concise root `AGENTS.md` as the operational entry point for coding agents: domain vocabulary pointer, source map, module/seam rules, authoritative commands, change checklist, privacy constraints, and where to place tests. Keep rationale in the Wayfinder tickets or later qualifying ADRs and keep owner-facing setup in `README.md`; do not duplicate the entire architecture into either file.
14. Call the scaffold verified when the full gate passes on the current macOS 26 host and the application is manually exercised there, while recording macOS 14 and 15 runtime checks as pending until real hosts exist. Before claiming a release supports all three versions, run the same manual checklist and full gate on each available version. A macOS 14 deployment-target build is compatibility evidence, but never a substitute for runtime evidence.

### Owner decision — round 2 (2026-08-28)

The owner accepted recommendations 8–14. For the three-target Xcode layout, the owner deferred to the recommendation because they are unfamiliar with macOS development. The owner also asked whether the verification contract ensures that Utilities are completely correct.

The repository still has no scaffold or tests, so no Utility behavior is implementation-verified yet. Earlier decisions require behavioral tests through every Utility's domain interface, while recommendation 10 specifies the representative scaffold matrix. Absolute correctness cannot be proven by tests; the remaining frontier defines a stronger, auditable meaning of contract-complete verification.

### Design tree — final round, awaiting owner answers (2026-08-28)

Recommendations presented to the owner:

15. Treat the settled contract for each Utility as its verification inventory. Every required behavior and stated failure mode must map to at least one focused automated test before that Utility is called complete. Test names and organization should make the mapping readable without maintaining a second duplicated requirements document.
16. Use independent oracles appropriate to the behavior: published standard vectors for standard formats, hand-reviewed fixtures for examples and diagnostics, and property tests for invariants such as round trips, normalization, alphabet membership, count, and length. Do not validate an implementation solely by comparing it with another call through the same implementation path.
17. Require a shared edge-case baseline where applicable: neutral empty input, Unicode, whitespace, malformed input, minimum/maximum configuration values, repeated values, and cancellation or stale-revision behavior. Add domain-specific boundaries from the owning Utility contract. Avoid probabilistic assertions about random quality and tight wall-clock performance tests; verify random invariants with deterministic adapters and assess responsiveness with explicit stress scenarios.
18. Use precise evidence language: a Utility may be called **contract-complete and verified** when its entire settled contract inventory passes against the named oracles and edge cases. Never claim that tests prove it "completely correct." Any discovered ambiguity or missing behavior returns to its owning product-contract decision rather than being silently invented in a test.

### Owner decision — final round (2026-08-28)

The owner accepted recommendations 15–18 without exceptions. The design tree is complete.

## Answer

Use one ordinary checked-in Xcode project with a shared scheme and folder-synchronized groups mirroring a feature-oriented source tree. Start with exactly three targets: the macOS application, one Swift Testing unit/integration test bundle, and one XCTest UI test bundle. Within the application target, keep explicit directories for the application shell, Utility History, platform adapters, shared presentation modules, and every source-defined Utility, with corresponding test directories. These remain modules with small interfaces even though they do not each require a separate build target. Do not add a workspace, project generator, or local Swift packages until a demonstrated need justifies the extra machinery.

### Authoritative commands and style

Check in `scripts/format`, `scripts/verify`, and `scripts/verify --full`. The default verification gate checks formatting, performs an Apple-Silicon Debug build, and runs every non-UI test. The full gate additionally performs a Release build and the focused UI suite. Both local agents and any later CI workflow invoke these scripts; the underlying `xcodebuild` commands remain documented but are not duplicated as competing recipes.

Use the `swift format` bundled with the selected Xcode toolchain and a checked-in configuration. Formatting changes remain an explicit action while verification uses lint-only mode. Treat compiler warnings as errors in first-party targets. Do not add SwiftLint unless concrete missing rules justify another dependency.

Keep Derived Data and result bundles under an ignored repository-local artifacts directory. Preserve failing result bundles for inspection and show the exact underlying command when a gate fails.

### Automated verification contract

Use Swift Testing for Utility domain behavior, Registry contracts, adapters, History persistence, concurrency and revision behavior, and snapshot schemas. Use XCTest/XCUIAutomation only for focused end-to-end shell and interaction-heavy flows.

The representative scaffold must test:

- Registry identity, metadata, category, inclusion, and search behavior;
- JSON valid and invalid input, formatting, minification, diagnostics, and agreed options;
- standard and URL-safe Base64, padding, UTF-8, invalid input, and agreed options;
- Identifier Generator format selection, validation, normalization, options, and batch behavior without asserting random literal values;
- Random String length, count, alphabet, exclusions, and related invariants through an injected deterministic random source;
- History retention, deterministic ordering, atomic-write failure, malformed-file and schema isolation, deletion, preview and restore, and the rule that restoration does not rerecord an entry;
- rejection of stale asynchronous results and snapshots.

The settled contract for every Utility is its verification inventory. Every required behavior and stated failure mode must map to a focused automated test before the Utility is called complete. Keep fixtures local to their owning Utility unless they express a genuine cross-module contract. Prefer published standard vectors, hand-reviewed fixtures, and property tests for round trips, normalization, alphabet membership, length, count, and similar invariants. An implementation cannot serve as its own sole test oracle.

Where applicable, cover neutral empty input, Unicode, whitespace, malformed input, minimum and maximum configuration values, repeated values, and cancellation or stale-revision behavior, plus domain-specific boundaries from the owning Utility contract. Use deterministic clocks, randomness, temporary directories, and predicate-based UI waits. Do not use probabilistic random-quality assertions, tight wall-clock tests, automatic retries, or a numeric code-coverage threshold. Coverage reports are diagnostic. A flaky test is fixed or removed from the gate with an explicitly tracked reason rather than hidden by retries.

The accurate completion claim is **contract-complete and verified**, never "proven completely correct." A behavioral ambiguity discovered during testing returns to its owning product-contract decision rather than being silently settled by the implementation or test.

### UI and accessibility verification

UI automation covers Library search and keyboard selection, workspace switching and session preservation, representative valid and diagnostic Utility states, History preview/restore and accessible deletion confirmation, and Settings persistence. Interactive controls receive stable accessibility identifiers, and tests cover labels, keyboard order, focus restoration, and the non-pointer confirmation route where XCTest can verify them credibly.

Keep system-wide shortcut activation, active-Space and full-screen placement, VoiceOver speech, Increase Contrast, Reduce Transparency, text scaling, and related visual behavior in a short manual release checklist. Do not convert those into brittle automation that implies stronger evidence than it provides.

### Supported macOS evidence

Every change must build with a macOS 14 deployment target. The initial scaffold may be called verified when the full gate passes and the application is manually exercised on the current macOS 26 host. Record macOS 14 and 15 runtime verification as pending until real physical or virtual hosts are available. Before claiming a release is runtime-verified across macOS 14, 15, and 26, run the full gate and relevant manual checklist on each version. Deployment-target compilation is compatibility evidence, not runtime evidence.

### Dependencies, CI, and agent guidance

Apple frameworks remain the default. Pin accepted Swift Package Manager dependencies in `Package.resolved`. Before adding one, record a concise review of its maintenance, license, offline and privacy behavior, binary impact, accessibility implications, and replacement seam. Do not add automated dependency-update infrastructure to the initial scaffold.

The authoritative continuous-verification surface is local until a Git repository and host exist. At that point, add the thinnest zero-out-of-pocket provider workflow that invokes the same verification script and introduces no CI-only behavior. The local gate remains usable and authoritative independently of hosted CI.

Add a concise root `AGENTS.md` containing the domain-vocabulary pointer, source map, module and seam rules, authoritative commands, privacy constraints, test placement, and change checklist. Keep owner-facing setup in `README.md`; keep detailed rationale in its owning Wayfinder ticket or a later ADR that satisfies the project's ADR threshold rather than duplicating the architecture across operational files.
