Type: spec
Status: ready-for-agent
Assignee: Unassigned
Blocked by: none
Title: Implement A — Precision Workbench and Graphite/Frappé themes

## Problem Statement

SofDevTool's current Workbench is functionally complete but visually relies on mostly default SwiftUI presentation. Its hierarchy, density, editor surfaces, catalog rows, and History inspector do not yet express the polished, compact Developer Toolbox shown by the approved A — Precision prototype. The application also has no product-owned theming boundary: Dark appearance is forced globally, while individual views select colors ad hoc or inherit platform defaults. This makes a coordinated visual refresh difficult and gives the owner no way to choose between the existing graphite direction and a softer Catppuccin direction.

## Solution

Implement the A — Precision prototype as the production visual and interaction baseline for the native macOS Workbench while preserving the settled Workbench information architecture and all Utility contracts. Introduce a small, app-owned, dark-only theming system with exactly two themes: Graphite and Catppuccin Frappé. Add a Theme picker to General Settings, apply changes immediately to the Workbench, Settings, Utility Launcher, shared Utility presentation, and History chrome, and persist the selection locally. Graphite remains the default and visual-continuity option; Catppuccin Frappé uses the official upstream palette and semantic color guidance.

## User Stories

1. As the owner, I want the production Workbench to use the A — Precision layout, so that the catalog, active Utility, and History remain visible in one dense working surface.
2. As the owner, I want the Workbench to feel intentionally designed rather than like unstyled default controls, so that I enjoy using it repeatedly throughout the day.
3. As the owner, I want Graphite and Catppuccin Frappé to be the only theme choices, so that theme selection stays simple and coherent.
4. As the owner, I want Graphite to remain the default theme, so that the update does not unexpectedly change the established appearance.
5. As the owner, I want to switch themes in General Settings, so that appearance configuration lives in the conventional macOS destination.
6. As the owner, I want a theme change to apply immediately, so that I can judge the result without reopening the app.
7. As the owner, I want my selected theme restored after relaunch, so that I do not need to configure it repeatedly.
8. As the owner, I want the selected theme to apply to the Workbench, Settings, and Utility Launcher, so that the application feels like one product.
9. As the owner, I want every Utility workspace to sit inside the same themed shell, so that switching Utilities does not cause unrelated visual changes.
10. As the owner, I want the Utility catalog grouped by its existing categories, so that visual polish does not change the established information architecture.
11. As the owner, I want each catalog row to show a symbol, name, and concise purpose, so that I can scan unfamiliar Utilities without opening them.
12. As the owner, I want the selected Utility to remain unmistakable in either theme, so that keyboard and pointer navigation stay easy to follow.
13. As the owner, I want Favorites to remain visible without dominating catalog rows, so that they help discovery without adding clutter.
14. As the owner, I want Library, Recent, and Favorites to remain compact toolbar scopes, so that I can change catalog context without leaving the Workbench.
15. As the owner, I want the central search field to remain prominent and keyboard accessible, so that opening a Utility stays fast.
16. As the owner, I want History, Utility Launcher, and Settings actions grouped at the trailing edge of the toolbar, so that global actions have one predictable location.
17. As the owner, I want the active Utility header to show its category, name, concise purpose, and Favorite action, so that the workspace has clear context.
18. As the owner, I want Utility-specific modes and actions to stay owned by each Utility, so that visual consistency does not flatten different operations into one generic workflow.
19. As the owner, I want input, result, diagnostics, and History preview surfaces to have clear visual separation, so that I can identify editable and read-only content at a glance.
20. As the owner, I want invalid input to remain visually distinct and remove stale output, so that the refreshed design preserves the existing correctness contract.
21. As the owner, I want success, warning, and error states to remain understandable without relying on color alone, so that either theme remains accessible.
22. As the owner, I want explicit Paste, Copy, Clear, and Restore actions to remain explicit, so that theming does not weaken privacy or operation boundaries.
23. As the owner, I want the History inspector to remain collapsible, so that I can give more room to a Utility when needed.
24. As the owner, I want the flexible Utility workspace to receive space before optional History content, so that smaller supported windows remain usable.
25. As the owner, I want keyboard selection, Return activation, focus movement, and shortcuts to behave as they do today, so that the visual refresh does not regress keyboard-first use.
26. As the owner, I want Increased Contrast, Reduced Transparency, larger text, and native focus indicators to remain respected, so that both themes work with macOS accessibility settings.
27. As the owner, I want standard macOS window behavior, menus, Dock behavior, and Launcher placement to remain unchanged, so that this work stays focused on Workbench presentation and theme selection.
28. As the owner, I want malformed or obsolete saved theme values to fall back safely to Graphite, so that a preference cannot prevent the app from opening.
29. As the owner, I want the theme implementation to remain completely local and dependency-free at runtime, so that SofDevTool preserves its offline and private product boundary.
30. As a future maintainer, I want semantic theme roles instead of scattered literal colors, so that a visual adjustment can be made once and applied consistently.
31. As a future maintainer, I want Utility domain code to remain unaware of the selected theme, so that presentation changes cannot affect Utility behavior or History snapshots.
32. As a future maintainer, I want the two palettes and their source to be documented, so that their intent and licensing remain auditable.

## Implementation Decisions

- Variant A — Precision is the production target. The implementation must reproduce its hierarchy and density using native SwiftUI and narrow AppKit integration; the throwaway HTML is a decision source, not code to port directly.
- Preserve the settled Workbench information architecture: a compact toolbar above a grouped catalog column, a flexible selected-Utility workspace, and a collapsible trailing History inspector. Recent and Favorites remain alternate catalog scopes rather than dashboards.
- The toolbar keeps the scope picker on the leading side, a central Utility search field, and History, Utility Launcher, and Settings actions on the trailing side. Existing shortcuts and accessibility identifiers remain stable unless the refreshed hierarchy requires an additive identifier.
- The catalog uses compact grouped rows containing the Utility symbol, display name, concise one-line summary, and a restrained Favorite indicator. Extend Utility Definition metadata with a short presentation summary; include the summary in registry search alongside existing names, categories, and aliases.
- The selected Utility header shows its symbol, category, display name, summary, and Favorite action. JSON-specific prototype controls such as “Show error” do not enter production. Modes, Paste, Copy, Clear, Swap, generation, and other commands remain owned by the concrete Utility workspace.
- Preserve persistent in-memory Utility Workspace Sessions. Changing theme, scope, search, History visibility, or selected Utility must not recreate or discard an already-open session.
- Prefer native split-view resizing. Use the prototype proportions as initial ideals: approximately 270 points for the catalog, at least 440 points for the workspace, and approximately 245 points for History. At constrained widths, History collapses before the catalog or active workspace becomes unusable.
- Keep all themes dark. Do not add Light, System, automatic scheduling, custom palette editing, or more Catppuccin flavors.
- Introduce one small presentation-owned theme boundary with a stable theme identity and a semantic palette. The semantic roles cover at least window background, secondary pane, raised surface, editor/input surface, separator, primary text, secondary text, subtle text, selection, app accent, success, warning, error, and code text.
- Theme identity has exactly two stable persisted values: `graphite` and `frappe`. Unknown, missing, or undecodable values resolve to Graphite without crashing. Graphite is the default for first launch and existing installations.
- Store only the selected theme identity in UserDefaults. Theme selection is presentation preference data, not Utility History, and must never enter a Utility Operation Snapshot.
- The selected theme is observable at application composition level and is injected through the SwiftUI environment. Views consume semantic roles rather than reading UserDefaults or switching directly on theme identity. Utility engines, snapshot types, History repository, and registry search logic do not depend on theme state.
- The General Settings tab gains an Appearance section with a labeled Theme picker containing “Graphite” and “Catppuccin Frappé.” Selection applies immediately and uses a stable accessibility identifier. The picker should offer small representative swatches only if they remain native, readable, and inexpensive; swatches are not required for completion.
- Apply theme changes live to every app-owned SwiftUI surface, including the Workbench, shared editor cards, diagnostics, History inspector, Settings, empty states, and Utility Launcher. New windows and a recreated Workbench use the current AppModel theme immediately.
- Continue forcing Dark Aqua at the application level so menus, sheets, panels, and system controls remain dark. App-owned chrome uses the chosen theme accent. This new explicit theme choice supersedes the earlier system-accent requirement for app-owned chrome, while standard macOS focus rings, accessibility emphasis, control behavior, and system surfaces remain native.
- Graphite is a product-owned palette based on the approved Precision prototype: background `#090B10`, secondary pane `#11151C`, raised surface `#171C25`, editor/input surface `#0D1118`, primary text `#F3F5F8`, secondary text `#9098A8`, subtle text `#697182`, accent `#7C83FF`, secondary accent `#4BD6D0`, success `#4BD18B`, warning `#FFB454`, and error `#FF6B7A`.
- Catppuccin Frappé uses the official upstream palette and semantic style guidance: Base `#303446`, Mantle `#292C3C`, Crust `#232634`, Surface 0 `#414559`, Surface 1 `#51576D`, Surface 2 `#626880`, Text `#C6D0F5`, Subtext 0 `#A5ADCE`, Subtext 1 `#B5BFE2`, Overlay 1 `#838BA7`, Mauve `#CA9EE6`, Lavender `#BABBF1`, Teal `#81C8BE`, Green `#A6D189`, Yellow `#E5C890`, and Red `#E78284`. Map these upstream colors to the semantic roles rather than exposing Catppuccin names throughout feature code.
- Use no theme package or runtime resource download. Check the required palette values into source, document the upstream Catppuccin palette version used, and add the MIT attribution to the existing third-party notice inventory.
- Theme-owned colors must maintain readable contrast for primary text, secondary text, selections, buttons, diagnostics, and disabled controls. State meaning must also use text, symbols, borders, or labels rather than color alone.
- Preserve macOS accessibility behavior: Increased Contrast strengthens separators and selection boundaries; Reduced Transparency removes nonessential translucent materials; text scaling and native focus indicators remain functional in both themes.
- Shared presentation components should carry the palette into text editors, result surfaces, diagnostic banners, Copy/Paste actions, and other genuinely shared chrome. Do not introduce a universal Utility layout or universal execute-input interface.
- The embedded Text Diff renderer retains its separately owned Pierre dark rendering theme in this increment. Its surrounding SwiftUI workspace and loading/error chrome use the selected app theme, and the boundary must remain open to a later renderer-theme mapping without importing renderer-specific types into the app theme system.
- Changing themes is a presentation action only. It does not open a Utility, alter Recents, record History, mutate Utility inputs/results, or produce a Utility Operation.
- The existing Graphite behavior remains usable even if Catppuccin attribution or palette validation fails during development; no network access is required at runtime.

## Testing Decisions

- Use the existing AppModel/UserDefaults boundary as the primary automated seam. This is the highest practical seam for default selection, mutation, persistence, invalid-value fallback, and isolation from Utility state without testing individual view implementation details.
- Add Swift Testing coverage proving that a fresh isolated UserDefaults suite selects Graphite, selecting Frappé publishes the new value and persists its stable identity, a new AppModel restores Frappé, and an unknown persisted identity falls back to Graphite.
- Test the semantic palette contract at its public presentation boundary. Assert the canonical values for the major Graphite roles and the official Catppuccin Frappé roles so accidental palette drift is caught. Do not test private Color construction helpers or every view modifier.
- Extend existing registry contract tests to require a non-empty concise summary for every Utility and to prove summaries participate in search. Continue testing immutable IDs, categories, aliases, symbols, and History metadata through the existing registry seam.
- Add one focused XCUITest through existing Settings and Workbench surfaces. It selects Catppuccin Frappé, verifies the Settings control and the visible Workbench report the same theme identity through stable accessibility values, closes and recreates or relaunches the app with an isolated preference domain, and verifies the selection remains Frappé.
- The UI test must also open the Utility Launcher after changing theme and verify that it reports the same theme identity. Do not compare screenshots or individual pixel colors in the automated suite.
- Keep existing catalog search, workspace switching, Launcher dismissal/selection, Settings opening, History, and Utility tests passing. The refresh must not require rewriting behavior-oriented tests merely because layout changed.
- Add a focused session-retention check if the layout refactor changes view identity: edit a Utility session, switch themes, and verify its current controls, input, diagnostic/result, and History state remain unchanged.
- Manually inspect both themes on the current macOS 26 host at the minimum supported window size and the default window size. Check catalog density, editor readability, selection, disabled controls, diagnostics, History preview/restore, Launcher, Settings, and window resizing.
- Repeat the existing owner-relevant display checks in both themes: Increased Contrast, Reduced Transparency, larger text, keyboard focus, and a non-default macOS accent. Record that app-owned chrome intentionally follows the selected theme accent while native system focus and controls remain legible.
- Run `scripts/format`, `scripts/verify`, and `scripts/verify --full`. Report runtime visual evidence only for the host actually exercised; macOS 14 and 15 deployment builds remain compatibility evidence rather than runtime theme verification.
- Good tests assert owner-visible behavior and stable contracts: selected identity, persistence, search metadata, session retention, and application-wide propagation. Avoid tests coupled to view nesting, exact split-view implementation, modifier order, or transient animation timing.

## Out of Scope

- Light appearance, following the system appearance, automatic theme scheduling, per-Utility themes, custom palettes, theme import/export, and additional Catppuccin flavors including Latte, Macchiato, and Mocha.
- Redesigning the Utility Launcher information architecture, Settings navigation, History storage model, Utility Registry architecture, or Utility Operation semantics.
- Adding tabs or a new persistent-session switcher from the C — Sessions prototype.
- Porting prototype-only demo controls, visible state panels, browser query parameters, or browser keyboard helpers into the native application.
- A universal Utility workspace layout or universal execute-input protocol.
- Retheming the embedded Pierre Text Diff document or modifying the vendored renderer bundle in this increment.
- Changing app lifecycle, Dock behavior, menus, global shortcut behavior, clipboard boundaries, History retention, privacy, or local-only product scope.
- Public distribution, appearance synchronization, telemetry, cloud storage, or downloading palettes at runtime.
- Pixel-perfect replication of browser rendering where native macOS controls provide the equivalent semantic behavior.

## Further Notes

- The local Workbench UX refresh prototype, Variant A — Precision, is the primary visual decision source. Implementers should use it to understand hierarchy, density, and relationships, then rebuild the result with native components and production error handling.
- The owner explicitly replaced the prototype's Catppuccin Mocha exploration with Catppuccin Frappé for the production specification. Mocha is not an accepted third theme.
- Catppuccin Frappé is a dark flavor. The official palette currently identifies itself as version 1.8.0; pin the copied values and attribution to the version verified during implementation rather than fetching latest values at runtime.
- Catppuccin palette data is MIT licensed. Preserve the copyright and permission notice in the application's third-party notices.
- This specification intentionally changes the previous “no appearance selector” decision only as far as selecting between two dark product themes. The dark-only platform boundary remains settled.
