# Candidate: one presentation owner, typed Utility bodies

## Problem

The approved proposal changes who owns presentation state as well as spacing. Workbench already owns catalog, selection, favorites, appearance and the only heading needed. Each workspace currently owns a duplicate `history_visible: bool`, initialized true, while `HistoryRecorder` separately controls collection. Putting History beside the single Workbench heading requires an explicit shared presentation boundary. Keep the concrete workspace sessions, snapshots, validation, execution and restore callbacks where they are.

Read-only grounding: `workbench.rs`, `ui.rs`, `preferences.rs`, `history.rs`, `settings.rs`, `launcher.rs`, `json_workspace.rs`, `utilities/random_string.rs`, `text_diff/mod.rs`; approved HTML; installed kit/component/base 0.6.6 sources. No product changes or native acceptance performed.

## Usage (caller's view)

These are design sketches, with new proposed interfaces called out by their definitions below, not compile-tested patches.

```rust
// Workbench constructs one presentation owner before the eager workspaces.
let layout = cx.new(|cx| WorkspaceLayout::load(layout_preferences, cx));
let json = cx.new(|cx| JsonWorkspace::new(
    window, cx, clipboard.clone(), history.clone(), layout.clone(),
));
// The one Utility heading owns the explicitly named visibility action.
layout.update(cx, |layout, cx| layout.set_history_visible(selected, visible, cx));
let recording = history.recording_state(selected.slug());
// Appearance stays authoritative in Workbench; Settings calls this method.
workbench.update(cx, |owner, cx| owner.set_appearance(mode, window, cx));
```

```rust
// JSON owns editor content, commands and kit split state, never shell policy.
h_resizable("json.editors")
    .with_state(&self.editor_split)
    .child(resizable_panel().size_range(px(220.)..px(2000.)).child(
        ui::pane(cx, "Input", input_actions, ui::multiline_editor(&self.input, false, "json.input"))
    ))
    .child(resizable_panel().size_range(px(220.)..px(2000.)).child(
        ui::pane(cx, "Result", result_actions, result_body)
    ));
// Narrow layout can use v_resizable with a distinct stable ID/state.
// Pane commands retain existing methods and disabled conditions.
```

```rust
// Settings changes collection, not visibility. Setter publishes status.
history.set_global_enabled(enabled, cx);
history.set_utility_enabled(id, enabled, cx);
// A workspace observes layout. Only the active Utility may mount its inspector.
match layout.read(cx).history_placement(Json::ID) {
    HistoryPlacement::Hidden => { /* no inspector */ }
    HistoryPlacement::Inline => { /* existing list + typed restore actions */ }
    HistoryPlacement::Sheet => { /* kit sheet, same retained list and callbacks */ }
}
```

## Shape

Use an explicit `Entity<WorkspaceLayout>` constructor parameter, passed through the existing registry construction boundary. This is UI presentation state, not a universal Utility context or executor. It costs constructor edits across Utilities but avoids process globals, duplicated preference writers and a shell interface containing one method for each Utility. Workbench remains the sole writer of navigation and measured workspace width; visibility changes from a sheet close go through the same entity.

```rust
// New workspace_layout.rs, app-owned; serialized records remain private.
pub struct WorkspaceLayout {
    active: UtilityId,
    workspace_width: Pixels,
    history_visible: BTreeMap<String, bool>,
    preferences: Option<HistoryLayoutPreferences>,
    // An optional preference-save diagnostic, no payloads or snapshots.
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryPlacement { Hidden, Inline, Sheet }

impl WorkspaceLayout {
    pub fn load(preferences: Option<HistoryLayoutPreferences>, cx: &mut Context<Self>) -> Self { unimplemented!() }
    pub fn history_visible(&self, utility: UtilityId) -> bool { unimplemented!() }
    pub fn history_placement(&self, utility_slug: &str) -> HistoryPlacement { unimplemented!() }
    pub fn set_history_visible(&mut self, utility: UtilityId, visible: bool, cx: &mut Context<Self>) { unimplemented!() }
    pub fn set_active(&mut self, utility: UtilityId, cx: &mut Context<Self>) { unimplemented!() }
    pub fn set_workspace_width(&mut self, width: Pixels, cx: &mut Context<Self>) { unimplemented!() }
}

// history.rs: a derived value, never a second persisted policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordingState {
    GlobalDisabled,
    UtilityDisabled,
    Paused,
    Enabled,
}
impl HistoryRecorder {
    pub fn recording_state(&self, utility_id: &str) -> RecordingState { unimplemented!() }
    pub fn set_global_enabled(&self, enabled: bool, cx: &mut App) { unimplemented!() }
    pub fn set_utility_enabled(&self, utility_id: &str, enabled: bool, cx: &mut App) { unimplemented!() }
}

// ui.rs: explicit app compositions, not a control library.
pub fn pane(cx: &App, title: impl Into<SharedString>, actions: impl IntoElement,
            body: impl IntoElement) -> impl IntoElement { unimplemented!() }
pub fn utility_icon(id: UtilityId) -> IconName { unimplemented!() }
```

`history_placement` returns Hidden for inactive Utilities and closed preferences, Inline for adequate workspace width, Sheet otherwise. The width threshold is one app-owned constant derived from two useful editor minima plus inspector width, tested at its boundary; width itself is measured from the workspace bounds rather than guessed from desktop resolution. Keep mode and drag sizes out of persistence unless separately requested. Each workspace retains its own kit `ResizableState` entities.

Store visibility in a separate `history-layout.v1.json` with one map of Utility slugs to booleans and a private versioned envelope. Missing entries default false. A single `WorkspaceLayout` writer atomically replaces it using the existing preference pattern. Keeping this file separate avoids stale Workbench theme/favorite writes overwriting layout state. No Utility input, result or History payload enters layout preferences. Load never opens a sheet for a Utility that is not active. Idempotent setters skip unchanged writes and notifications.

`recording_state` derives global override first, then per-Utility/default policy, then store pause, otherwise Enabled. Existing recording defaults and `record` semantics remain unchanged; JWT stays opted out. A paused badge does not silently retry. Both setters persist and notify status. Workbench subscribes to status once so its heading updates when Settings changes collection or a workspace records unsuccessfully. `retry_recording`, `clear_utility` and `clear_all` must publish status after their existing storage notifications because they can alter pause state. Do not call the existing synchronous per-Utility callbacks from inside a workspace mutation when it would recursively update that same entity.

### Module map and retained ownership

- `workbench.rs`: compact global bar, full-width ghost catalog rows, one heading with favorite action, derived recording label and Show/Hide History. Own layout entity, selection and width updates. Expose appearance getter/setter for Settings rather than duplicating theme state. Retain current `Root` mounting, focus handles, shortcuts, `AnyView` construction and session cache.
- `workspace_layout.rs`: presentation placement and visibility persistence coordination only. No collection methods, restore methods or domain engine handles.
- `preferences.rs`: private layout envelope and atomic load/save; existing files and defaults unchanged.
- `history.rs`: derived effective recording state and complete status notification boundary. Keep retention at 25 and snapshot contents untouched.
- `ui.rs`: `pane` adds pane-local action placement while reusing kit controls. Keep current `panel` where still needed; remove obsolete helpers only after migrating callers. Shared `utility_icon` centralizes the UI mapping without making the domain Registry depend on kit types. Preserve `multiline_editor` exactly.
- `json_workspace.rs`: direct kit Select for indentation, Checkbox for Sort keys, existing segmented mode; Paste/Clear in Input header and Copy in Result header; retained resizable editor state; History presentation observer. The Select state translates its chosen value to `Indentation` at the view boundary and is synchronized during restore without resubmitting or recording.
- `utilities/random_string.rs`: remove local title and vertical `flex_1` from custom-character configuration; checkboxes call existing configuration-save path without generation; numbers retain NumberInput; Paste beside Custom; Copy all/Clear in result header. Results alone consume remaining height.
- `text_diff/mod.rs`: horizontal resizable old/new panes inside vertical resizable sources/comparison; pane-local Paste/Copy; preserve mode, renderer generation and History restoration. Sheet lifecycle hides the native renderer and restores its active state on dismiss, Utility switch or Launcher activation.
- Other Utility workspace files: same constructor presentation handle, no misleading History toggle and default-closed behavior; remove repeated Utility title where present. Preserve per-Utility layouts instead of forcing JSON's arrangement on every Utility.
- `settings.rs`: appearance selector and compact Switch rows, global override visibly reflected; build details hold full profile/revision/dirty identity. Settings still routes all operations to their current owners.
- `launcher.rs`: same kit icon mapping and ghost rows, preserve existing selection, arrows, Enter/Escape and activation lifecycle.

### Verified dependency surface

`gpui_kit::component` exposes the installed `gpui-component-0.6.6`; its `resizable` module reexports `gpui-base-0.6.6` `h_resizable`, `v_resizable`, `resizable_panel`, `ResizableState` and panel types. `ResizablePanelGroup::with_state`, `axis`, `child`, `on_resize` and `ResizablePanel::size_range` exist. This supports real kit dragging without an app resize engine.

Component Checkbox has `checked`, `label`, `accessibility_label`, `on_change`; Switch has matching checked/change methods. Select has retained `SelectState`, `SelectEvent`, `set_selected_value`, `selected_value`, `Select::new`, `cleanable(false)` and `accessibility_label`. Existing app NumberInput and TabBar usage already grounds numeric/segmented controls. IconName lives in component `icon.rs` and uses kit assets; pick actual variants from that file, not arbitrary SVG paths.

`WindowExt::open_sheet`, `open_sheet_at`, `has_active_sheet`, `close_sheet` exist. Sheet supports `size`, `resizable`, `overlay`, `overlay_closable`, `on_close`. Kit Root retains and restores the prior focus handle. Root must remain the actual window root. Restore-to-editor still needs application attention: the click on the Show History button can have moved focus before open_sheet captures it. Capture the editor focus before the opener steals it, or retain the last focused editor per Utility, then restore it explicitly after dismissal. Native WebDiff focus needs its existing adapter.

Sheet mounting is a transition, not a render-time repeated side effect. Observe layout changes and defer a transition until the current entity update finishes; keep a local runtime placement/owned-sheet flag. Width changes never write visibility preferences. Close the previously owned sheet before selecting another Utility. Entering narrow mode while an inline inspector is open moves that explicit open state into a sheet; starting narrow with default closed does nothing. Sheet dismissal closes the preference and cannot immediately reopen itself. The sheet builder renders live list state, not a captured list snapshot. At most the active Utility owns a sheet.

## Tradeoffs accepted

- Accept a constructor parameter migration across workspace files in exchange for a visible single owner and no global UI state.
- Accept local typed restore/render callbacks in every workspace in exchange for preserving snapshot/version semantics and avoiding a universal workspace abstraction.
- Accept one additional presentation preference file in exchange for avoiding competing writers of existing Workbench preferences.
- Keep favorite affordance always reachable and subdued if hover-or-focus reveal is awkward in the installed kit; never choose hover-only discoverability.

## Alternatives considered

1. Workbench renders History itself through a universal `Workspace` trait providing rows, restore, clear, body and layout. It hides placement deeply, but exposes domain restoration through erased callbacks, widens the registry contract and risks another app component framework. Rejected: the presentation gain does not justify moving typed snapshot behavior out of Utility owners.
2. Every workspace owns its visibility preference file, header and sheet independently. It keeps constructors unchanged and isolates writers, but replicates header, width policy, appearance interaction and active-sheet lifecycle across all Utilities. Callers must learn and maintain all presentation rules. Rejected: superficially smaller edits produce a weaker interface and future divergence.
3. A global presentation entity avoids constructor changes, but every Utility gains an implicit application initialization dependency. The explicit entity candidate makes ownership and isolated testing clearer for modest signature churn.

## Synthesis decision

Candidate pending parent synthesis. Prefer this explicit shared presentation owner if the implementation includes heading-level History affordances for all Utilities. If scope deliberately leaves toggles local, the constructor migration can be avoided, but that would diverge from the approved single-heading proposal.

## Risks and verification

Questions to resolve through implementation evidence, not a mandatory new approval gate: Does the native WebView remain hidden under sheets/dialogs at every resize? Does focus return to the prior editor after a button-opened sheet? Does restoring a snapshot update retained Select/Checkbox state without scheduling a new operation? Do all hidden retained sessions redraw on status/layout changes without reentrant updates?

Focused tests: layout default-closed and per-Utility round trip; width/selection placement table; visibility changes do not alter policy or entries; status precedence for global disable, Utility disable, paused and enabled; notifications after Settings toggle/retry/clear; existing JSON/Random String/Text Diff exact restore and non-recording tests. One GPUI flow should open/close the narrow sheet, change Utility with it open, then return, verifying selection and session survive. Preserve existing grapheme tests.

Run `make format` and `make verify`, then native artifact checks at 1000x700, 1280x800 and large windows in System/Light/Dark, History open/closed, keyboard-only focus, long labels, valid/error/empty/result states, copy feedback and Text Diff drag/focus. Root gate success is not native acceptance; specifically exercise text editor undo/redo, Unicode partial selections and native renderer geometry.

## Next implementation step

Implement and test effective recording state plus the single-owner layout preferences first, then wire the Workbench heading and JSON as the first full vertical slice before adapting the remaining typed workspaces.
