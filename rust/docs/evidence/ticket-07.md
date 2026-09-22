# Ticket 07 — navigation, scopes and themes

## Scope

A compact toolbar, a grouped catalog with Library / Recent / Favorites scopes,
a flexible selected workspace and a collapsible trailing History, with persisted
favorites, recents and theme.

## Implementation

- The Workbench sidebar has scope buttons, a search field, and (in Library) a
  catalog grouped by category. Recent and Favorites are catalog filters backed
  by persisted slug lists; opening a Utility moves it to the front of Recents
  (capped at 8). Each row has a favorite toggle and a stable focus handle.
- `WorkspacePreferencesData { theme, favorites, recents }` persists atomically
  in the fresh Rust namespace; missing or malformed data yields safe defaults
  (Graphite, empty favorites/recents).
- The theme is process-wide (`set_active_theme`/`active_theme`), so every
  semantic token call site is theme-aware without threading a context argument.
  Graphite is the default and Catppuccin Frappé remains available; the header
  theme control switches between them and re-renders without recreating
  sessions or changing Recents.
- Each Utility workspace keeps its own collapsible History (History: on/off),
  so switching catalogs, scopes, theme or Settings preserves sessions and focus.
- The component gallery now demonstrates the real History list/select,
  destructive `HoldButton`, confirmation banner, diagnostics, copy feedback,
  empty state and theme switch, and still builds without the application crate.

## Verification

- Unit tests cover the workspace-preferences round trip, safe defaults on
  malformed data, and registry search/aliases.
- Gate: `rust/scripts/verify --full` exit 0, including the Debug and Release
  gallery build.
- Native catalog/scope/theme/session-preservation scenarios are pending final
  host verification (see ticket 22 evidence).

## Limits

macOS 14/15 runtime unverified. Full VoiceOver parity remains deferred.
