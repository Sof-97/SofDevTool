Type: prototype
Status: resolved
Assignee: Codex
Blocked by: 02

## Question

Which native-feeling information architecture makes the growing Utility catalog fast to browse and search in a traditional macOS window? Prototype and validate navigation, Utility categories, favorites or recents if useful, keyboard movement, the searchable Utility Launcher opened by a configurable global shortcut, empty/error states, and the relationship between the main window and Settings.

## Comments

### Prototype ready for owner review — 2026-08-21

- Asset: [Main window and Utility Launcher prototype](../prototypes/main-window-prototype.html)
- Open the file directly in a browser. Use the floating arrows or `?variant=A`, `?variant=B`, and `?variant=C` to compare Library/browse-first, Home/search-first, and Workbench/keyboard-first structures.
- In every variant, try search, the Utility Launcher (`⌘K` in the browser prototype), Settings, a no-results query, and the JSON error-state control. The Launcher supports arrow keys and Return; the main Utility list supports `J`/`K`.
- This is a throwaway, in-memory prototype. It intentionally does not select a winner or settle the ticket without the owner's reaction.

### Owner verdict — 2026-08-21

The owner selected Variant C, the Workbench/keyboard-first structure.

## Answer

Use Variant C as the information-architecture baseline: a dense native macOS workbench that keeps discovery and the active Utility visible together.

- The main window has a compact toolbar with Library, Recent, and Favorites scopes; one central Utility search field; and direct Utility Launcher and Settings actions. Library is the default scope.
- The content uses a two-column split view. The left column is a searchable, keyboard-navigable Utility list grouped by category. Categories organize the catalog but do not introduce a deeper navigation hierarchy. The right column is a persistent workspace for the selected Utility.
- Selecting another Utility replaces only the workspace content. The window, current catalog scope, and search context remain stable. The app may restore the last selected Utility when reopening, subject to the separately decided persistence and privacy rules.
- The Utility Launcher is a distinct, transient overlay opened by a configurable global shortcut. It searches the same registry as the main window, supports native arrow-key movement and Return to open, and opens the chosen Utility in the main window rather than creating a second execution surface.
- Recent and Favorites are useful alternate views of the same Utility list, not separate dashboards. Utility History entries remain inside their owning Utility or the History view; they do not crowd the default Library.
- Settings is a separate native destination launched from the toolbar. Opening it does not discard or replace the selected Utility and its in-memory state.
- An empty search shows a neutral no-results state with a clear-search action. Utility-specific diagnostics remain in the workspace and replace stale results, following the shared Utility contract.
- Production keyboard behavior should follow native macOS conventions: focus search, move through visible results with arrow keys, open with Return, and preserve sensible focus when the selected Utility changes. Prototype-only `J`/`K` controls and the visible state panel do not carry into the app.

Primary source: [Main window and Utility Launcher prototype](../prototypes/main-window-prototype.html), Variant C.
