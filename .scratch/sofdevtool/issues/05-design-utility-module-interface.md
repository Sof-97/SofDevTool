Type: grilling
Status: resolved
Assignee: Codex
Blocked by: 01, 02, 03

## Question

What is the smallest deep module interface that lets agents add source-defined Utilities consistently without flattening their distinct interactions? Decide the registry metadata, categories and search terms, command and view boundaries, validation and error model, clipboard actions, history participation, test contract, and extension seams required by the selected stack and validated prototype.

## Comments

### Design tree — round 1, awaiting owner answers (2026-08-21)

The current frontier covers five independent choices: the source-level unit of extension, the registry's catalog metadata, whether execution is standardized, the event by which a Utility participates in History, and the minimum test contract. The separately mapped History ticket retains ownership of persistence, retention, deletion, and sensitive-data policy.

Recommendations presented to the owner:

1. Make each Utility a source-defined feature module with a stable definition and SwiftUI workspace, registered explicitly at compile time; do not create a runtime plugin or discovery mechanism.
2. Let the central Utility Registry own only catalog concerns: stable ID, display name, category, search aliases, symbol, history capability/default, and workspace factory. Use a small fixed category vocabulary and search aliases rather than tags.
3. Do not force unlike Utilities through a universal execute(input) interface. Each Utility owns its state, commands, validation, diagnostics, and domain engine; the shell sees only its definition/workspace and narrow shared capabilities.
4. Have a Utility report one completed valid Utility Operation through a typed History recorder carrying a Utility-owned, versioned snapshot. Invalid/intermediate states emit nothing. The History module later decides whether and how the snapshot is persisted.
5. Require registry contract tests plus behavioural tests through each Utility's domain interface; require focused SwiftUI/UI tests only for shared shell behavior and interaction-heavy Utilities, not a bespoke UI suite for every Utility.

### Owner decision — round 1 (2026-08-21)

The owner agreed with all five recommendations.

### Design tree — round 2, awaiting owner answers (2026-08-21)

Recommendations presented to the owner:

6. Use four fixed primary categories: Format & Convert, Encode & Inspect, Generate, and Compare & Test. Every Utility has exactly one primary category; alternate discovery terms are search aliases rather than secondary categories or tags.
7. Give every Utility a short immutable ID and every Utility Operation Snapshot its own integer schema version. Display names and categories may change without changing identity; snapshot migrations remain owned by the Utility.
8. Keep one in-memory workspace session per opened Utility for the application's lifetime so switching Utilities preserves unfinished controls, input, and results. Cross-launch payload restoration occurs only through Utility History; ordinary control preferences may restore separately under the already settled privacy rule.
9. Share only a small diagnostic envelope—severity, human-readable message, and optional source location/range. Concrete parsing and validation error types remain private to their Utility, and no stale result accompanies an error.
10. Put only genuine cross-cutting adapters in the workspace context: explicit Clipboard access and recording a completed Utility Operation Snapshot. Inject clocks, randomness, and other deterministic test dependencies inside only the Utility modules that need them; do not grow a service-locator-style environment.
11. Type-erase the concrete SwiftUI workspace only at the heterogeneous Utility Registry seam. Concrete Utility definitions, views, models, and domain engines remain strongly typed inside their modules and tests.

### Owner decision — round 2 (2026-08-26)

The owner agreed with recommendations 6–11 without exceptions.

### Design tree — final round, awaiting owner answers (2026-08-26)

Recommendations presented to the owner:

12. A Utility owns its full workspace content and Utility-specific commands, including Copy, Clear, Swap, Run, and mode controls. The application shell owns only navigation, selection, window-level search, Favorites/Recents, Settings, and presentation of Utility History. Shared SwiftUI controls are reusable implementation modules, not requirements added to the Utility Definition interface.
13. Express History participation as a three-state capability on each Utility Definition: supported and enabled by default, supported and disabled by default, or unsupported. The first-release Utilities support History; JWT Decoder defaults disabled. The next ticket owns owner overrides, retention, storage, deletion, and restoration failure behavior.
14. Favorites and Recents store immutable Utility IDs as application-level state. Utilities neither know nor emit favorite/recent state; opening a Utility updates Recents in the shell regardless of whether a Utility Operation completes.
15. Each Utility session owns asynchronous work, cancellation, debouncing, and revision checks. Only the latest still-current request may update the visible result or emit a Utility Operation Snapshot. This invariant belongs to the Utility module contract, but no universal task-runner interface is imposed.

### Owner decision — final round (2026-08-26)

The owner agreed with recommendations 12–15 without exceptions. The design tree is complete.

## Answer

Use a small compile-time Utility Registry whose entries are type-erased only at the heterogeneous SwiftUI catalog seam. Each source-defined Utility remains a cohesive, strongly typed feature module with its own definition, workspace, session state, domain engine, commands, validation, and tests. There is no runtime plugin discovery and no universal `execute(input)` abstraction.

### Utility Definition and Registry

Every Utility Definition provides only:

- a short immutable Utility ID;
- display name;
- exactly one primary category;
- search aliases;
- an SF Symbol or equivalent catalog symbol;
- History capability and default;
- a factory for its type-erased SwiftUI workspace.

The four fixed categories are **Format & Convert**, **Encode & Inspect**, **Generate**, and **Compare & Test**. Alternate discovery terms are aliases, not tags or secondary categories. Registry contract tests enforce unique stable IDs, valid metadata, catalog inclusion, and expected search/category behavior.

Display names, symbols, aliases, and categories may evolve without changing Utility identity. Favorites, Recents, History, and restoration all refer to the immutable Utility ID.

### Workspace and application-shell seam

The application keeps one Utility Workspace Session per opened Utility for its process lifetime, so navigating away preserves unfinished controls, input, diagnostics, and results. Cross-launch payload restoration may occur only through enabled Utility History; ordinary control restoration follows the separately settled privacy rule.

Each Utility owns its entire workspace and all Utility-specific commands, including Copy, Clear, Swap, Run, modes, and focus behavior. The shell owns the Utility Registry, selection, navigation, catalog search, Favorites, Recents, Settings, and History presentation. Opening a Utility updates shell-owned Recents even when no Utility Operation occurs. Reusable SwiftUI controls may reduce duplication, but they are implementation modules rather than requirements added to the Utility Definition interface.

The workspace context contains only two genuine cross-cutting adapters: explicit Clipboard access and a History recorder to which a completed Utility Operation Snapshot can be offered. Clocks, randomness, and other deterministic test dependencies are injected internally only into Utilities that need them. The workspace context must not become a service locator.

### Results, diagnostics, and asynchronous work

Utilities keep their concrete result and error types private. They share only a small presentation-level diagnostic envelope containing severity, a human-readable message, and an optional source location or range. Empty input remains neutral. An invalid current input displays its diagnostic without a stale result.

Every Utility Workspace Session owns its live-work debounce, explicit asynchronous tasks, cancellation, and input-revision checks. Only the latest still-current request may update the visible result or emit a Utility Operation Snapshot. This is a module invariant, not a universal task-runner interface.

### History participation

One completed valid Utility Operation may produce one Utility-owned, versioned Utility Operation Snapshot. Intermediate edits and invalid attempts produce none. Every snapshot carries the immutable Utility ID and its own integer schema version; the owning Utility also owns decoding and migration of its previous snapshot schemas.

Each Utility Definition declares one of three History capabilities:

- supported and enabled by default;
- supported and disabled by default;
- unsupported.

All first-release Utilities support History. JWT Decoder is supported but disabled by default. The separate History ticket owns user overrides, retention, persistence representation, deletion, stronger protection, restoration, and failure behavior; the recorder seam does not pre-decide those policies.

### Verification and extension contract

Each new Utility must add its definition to the explicit registry, satisfy registry contract tests, and provide behavioral tests through its domain interface. History-participating Utilities test snapshot emission and schema decoding/migration through their own interface. Focused SwiftUI or UI tests are required for shared shell behavior and interaction-heavy Utilities, not mechanically for every Utility.

Concrete definitions, views, models, and domain engines stay strongly typed in their modules and tests; only the registry's workspace factory erases the view type. Clipboard and History have production and test adapters because those are real seams. No additional interface is introduced until behavior actually varies across it.
