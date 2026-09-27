# Selected architecture

Base: Astra candidate in astra.md. Independent Sol cross-judge in judge.md scored it 23/25 against 19/25 for sol.md. Both converge on a single presentation owner with Utility-owned typed content. The reactive mechanisms differ: GPUI Entity observation versus a custom Rc subscriber registry.

Choose the GPUI Entity. It avoids duplicating the framework's observer lifecycle. Keep its interface typed with UtilityId; serialize slugs only at the preference boundary. Persist only visibility in a separate layout preference file, default closed. Derive responsive placement; resize does not persist or broadcast unchanged placement. Retain Utility-specific history rows, restoration, controls and snapshots.

Graft explicit non-payload preference errors, unknown-slug filtering, session-only split sizes and accessible icon action checks from Sol. Status derives from existing recording policy and paused storage without changing capture behavior. Status observers update the shell when Settings changes policy.

Keep active-sheet transitions coordinated and deferred, outside render. Use actual kit controls, resizable panels and sheets. Preserve the prior editor focus when the History opener would steal it. Text Diff's native WebView must hide beneath a sheet and recover when it closes.

Reject a universal workspace/execute API and a second UI component library. Keep the multiline grapheme deletion adapter and the specialized Text Diff renderer.

## Implementation slices

1. Typed layout preferences and status derivation, compact Workbench shell, constructor migration and unambiguous History action for all consumers.
2. JSON pane-local actions, semantic options and splits; Random String compact configuration and flexible results.
3. Text Diff split sizing and overlay behavior, Launcher rows, Settings appearance and compact recording rows, exact build details.

Each slice runs focused tests and cargo check before proceeding. The final root gate and native validation are mandatory. Parent owns review and native validation; one Sol implementation owner writes product source. Keep source changes separate from parent-owned verification records.
