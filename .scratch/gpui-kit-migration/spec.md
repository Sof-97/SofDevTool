# Complete GPUI Kit migration

**Status:** ready-for-agent
**Type:** task

## Problem Statement

The Developer Toolbox currently uses GPUI Component through sofui, an independently versioned custom component library. Custom controls, interaction helpers and theme palettes duplicate responsibilities the owner wants GPUI Kit to own. Maintaining that layer prevents consistent adoption of upstream controls and appearance. The owner wants its complete removal while retaining all existing Utility capabilities and privacy behavior.

## Solution

Replace the custom component and theme infrastructure with the latest stable GPUI Kit in one complete delivery. Adopt the kit's appearance and standard interactions while preserving the organization, functionality and local-only nature of the Developer Toolbox. Keep app-owned Utility compositions and the specialized Text Diff renderer as explicit boundaries, not as a replacement component library.

Offer System, Light and Dark appearance using official Catppuccin Latte and Catppuccin Frappe presets. Remove sofui as a separate product, including its gallery and dedicated consumer infrastructure.

## User Stories

1. As an owner, I want to use GPUI Kit controls throughout the Developer Toolbox, so that interaction and appearance remain consistent.
2. As an owner, I want to keep every existing Utility available, so that the migration does not remove capabilities.
3. As an owner, I want to receive one complete replacement, so that I never have to accept an intermediate mixed implementation.
4. As an owner, I want to retain the familiar organization of Utilities, so that I can find my existing workflows.
5. As an owner, I want to search and open Utilities through the Utility Launcher, so that keyboard access remains efficient.
6. As an owner, I want to retain Launcher shortcuts and window activation, so that the app remains accessible from other applications.
7. As an owner, I want to retain favorites and recent Utilities, so that my navigation preferences survive the migration.
8. As an owner, I want to switch between System, Light and Dark appearance, so that appearance follows my preference.
9. As an owner, I want to use Catppuccin Latte in Light mode and Catppuccin Frappe in Dark mode, so that both appearances use official kit presets.
10. As an owner, I want to see System mode react to macOS appearance changes, so that the app stays consistent with my desktop.
11. As an existing owner, I want to have my previous custom theme preference mapped to Dark, so that the migration preserves my previously dark appearance.
12. As a new owner, I want to start in System mode, so that initial appearance matches macOS.
13. As an owner, I want to retain the selected appearance after restarting, so that I do not have to configure it repeatedly.
14. As an owner, I want to use every Utility and theme offline, so that the Developer Toolbox remains local-only.
15. As an owner, I want to edit and select text with standard kit controls, so that editing is consistent across Utilities.
16. As an owner, I want to retain correct Unicode editing, undo and redo, so that editing does not damage text or lose expected recovery.
17. As an owner, I want to retain diagnostics, limits and invalid-input behavior, so that each Utility remains trustworthy.
18. As an owner, I want to copy and paste only through explicit actions, so that Clipboard access remains intentional.
19. As an owner, I want to switch Utilities without losing the Utility Workspace Session, so that ongoing work remains available.
20. As an owner, I want to record only completed valid Utility Operations in Utility History, so that invalid and intermediate edits do not pollute History.
21. As an owner, I want to restore a Utility History Entry without recording it again, so that restoration does not duplicate operations.
22. As an owner, I want to retain History collection and retention settings, so that my privacy preferences remain effective.
23. As an owner, I want to confirm destructive actions through a standard accessible dialog, so that accidental deletion remains preventable.
24. As an owner, I want to cancel a destructive confirmation without changing data, so that I can reconsider safely.
25. As an owner, I want to navigate controls, lists and dialogs with the keyboard, so that mouse use is optional for supported workflows.
26. As an owner, I want to keep Text Diff capabilities with theme-aware rendering, so that comparison remains useful in both appearances.
27. As an owner, I want to see domain-specific results such as color previews, so that standard controls do not remove specialized information.
28. As an owner, I want to retain separate Debug and Release data, so that development checks do not affect my normal app.
29. As an owner, I want to be notified when an upstream control cannot preserve an existing behavior, so that I decide how to handle incompatibilities.
30. As a maintainer, I want to remove sofui and its dedicated product infrastructure, so that there is no second component library to maintain.
31. As a maintainer, I want to use one upstream theme authority, so that application code does not maintain a competing palette.
32. As an owner, I want to receive explicit automated and native validation evidence, so that completion claims reflect what was actually exercised.

## Implementation Decisions

- The verified stable target at planning time is GPUI Kit 0.6.6. The current application already pins GPUI Component 0.6.6 and GPUI 0.3.6; this is primarily an adoption and removal effort, not merely a version bump. Recheck the latest stable release at implementation start and record the exact version and compatibility findings; do not silently accept behavior losses in a newer release.
- Use the official GPUI Kit facade and supported component/theme interfaces directly. Application startup owns runtime initialization, assets, root mounting and process hooks. Do not create a successor wrapper library or local component fork.
- Remove sofui, its public component API, its independent version/release intent, gallery, sample consumer and dedicated isolation tooling. Preserve relevant external behavior coverage in application tests before deleting obsolete library tests.
- Standard kit controls replace custom buttons, hold controls, confirmation bars, lists, segmented controls, numeric controls, text fields/editors, feedback, banners and other shared controls. Where no one-to-one control exists, compose supported kit controls within the owning application view. Do not recreate a generic custom control system.
- Keep app-specific workspace composition, Utility wiring, result presentation and color previews. Use the kit's theme tokens for application chrome; data colors remain domain output.
- Application and core retain Utility execution, validation, strong Utility types, snapshots, persistence, History policy and lifecycle. Type erasure remains limited to workspace construction in the Utility Registry; no universal execute-input interface is introduced.
- Standard text controls must preserve observable editing and restore behavior, including Unicode deletion, selection, undo/redo, explicit Clipboard access and the distinction between user edits and programmatic restoration. The existing wrapper's implementation is not a requirement, its externally relevant behavior is.
- Preserve asynchronous result freshness and Utility Workspace Sessions. Restoring a snapshot must not generate a duplicate Utility Operation or permit an obsolete completion to overwrite restored state.
- Replace press-and-hold destructive interactions with standard kit confirmation dialogs. Preserve the action's scope, keyboard usability, cancellation behavior and visible reconciliation after deletion.
- The kit's Theme and ThemeRegistry are the sole theme authority. Load the unmodified official Catppuccin presets as bundled assets for offline use; no custom Graphite palette or parallel palette state remains.
- Light selects Catppuccin Latte; Dark selects Catppuccin Frappe. System follows macOS appearance, including changes while running. Application preferences store the user's mode; the kit resolves and applies the effective theme.
- Map existing Graphite and Catppuccin preferences to Dark/Frappe. New installations default to System. Preserve unrelated preferences and maintain Debug/Release isolation. Theme migration must be repeatable without overwriting a subsequently saved mode.
- Retain the specialized Text Diff renderer and native embedding as the explicit exception. Its surrounding controls move to the kit and its effective light/dark styling follows the kit. This does not authorize replacing the renderer or introducing a new experimental WebView adapter without necessity.
- Preserve current History collection, retention, deletion, restore and privacy behavior. Never log, sync, index or export History payloads.
- Deliver the migration atomically. Tickets are work packages in one coordinated integration effort, not independently shipped or accepted intermediate product states. Do not add compatibility wrappers, a feature-flagged old UI, or an expand-contract rollout. Source edits are necessarily sequential; only the complete integrated result is promised to build and pass the final gate.
- Notify the owner of any demonstrated upstream regression or missing capability that prevents the agreed behavior. Describe the affected workflow and evidence; do not silently keep custom controls, reduce scope or claim completion. An unresolved incompatibility remains a completion blocker while unaffected planning/work can continue.

## Testing Decisions

These testing seams and the ticket breakdown were approved by the owner.

- Prefer the existing application interaction boundary: edit or generate, observe result/diagnostic, settle a Utility Operation, inspect History state and restore. Test external behavior rather than component types, internal widget structure or theme implementation details. Do not introduce a new broad testing abstraction.
- Reuse the existing Utility-owned core tests and retained contract vectors for execution, limits, validation and snapshots. UI migration should not rewrite independent expected results merely to obtain green tests.
- Reuse application GPUI tests for Settings confirmations, visible and hidden History reconciliation, preferences and restoration. Replace hold-specific expectations with the agreed confirmation-dialog behavior.
- Retain meaningful text interaction scenarios currently covered through sofui at the application boundary: Unicode graphemes, selection, undo/redo, user change versus restore, focus and keyboard navigation. Do not retain a dummy sofui consumer to preserve obsolete test structure.
- Cover theme preference migration and restart behavior with isolated temporary data. Exercise fresh System mode, both previous preference values, explicit Light/Dark, system appearance changes and unchanged unrelated preferences.
- Check each of the fifteen Utilities through representative valid, invalid and relevant boundary inputs; prioritize the changed interaction and lifecycle paths rather than duplicating every core vector in UI tests.
- Verify Text Diff assets remain reproducible and packaged offline, both appearances update, and native focus, keyboard/Clipboard behavior, resize and recovery still work.
- Run the relevant maintained root gate for the final integrated tree, including make verify-full for Release, renderer and packaging coverage. Update that gate to remove retired sofui products while retaining app validation.
- Record direct native scenarios separately from automated checks: Workbench, Launcher activation and reopening, Settings dialogs, editor interactions, History restoration, themes and Text Diff. Use isolated profiles and do not delete personal data for acceptance.
- Report OS/version, build revision and scenarios actually observed. A build, automated GPUI test, temporary install or run on a newer macOS does not establish macOS 14/15 native acceptance. Missing evidence remains explicitly missing.

## Out of Scope

- New Utility capabilities, revised domain limits, revised History/privacy policy or migration of legacy Swift product data.
- Publishing a new sofui release, maintaining a compatibility component library, or creating a local fork of kit controls.
- Pixel-for-pixel preservation of custom appearance, custom theme editing, and a theme marketplace.
- Replacing the specialized Text Diff engine or promising a fully GPUI-native diff view.
- Intermediate releases, independently accepted partial migrations, or keeping an old UI behind a feature flag.
- New platform support, automatic installation over the owner's normal app, release publication or unrelated refactoring.

## Further Notes

Product decisions, testing seams and the ticket breakdown were approved in the conversation. The specification and tickets are published as ready-for-agent. Publication does not begin source implementation.

No current ADR directory was present during planning. The new direction intentionally supersedes the existing documented intent to publish sofui independently; maintained ownership guidance and commands must be reconciled during implementation.

References: [GPUI Kit 0.6.6](https://docs.rs/crate/gpui-kit/0.6.6), [component catalog](https://docs.rs/gpui-component/0.6.6/gpui_component/), [themes](https://gpui-kit.com/component/theme/), [official Catppuccin presets](https://raw.githubusercontent.com/longbridge/gpui-kit/v0.6.6/themes/catppuccin.json).
