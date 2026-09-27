# Candidate: Workbench-owned chrome, Utility-owned content

## Problem

The approved proposal changes navigation density and work area while preserving the Utility contracts. Today `Workbench` owns the catalog and one generic Utility heading, but each of 15 retained workspace sessions owns its own controls, History view, and `history_visible: bool` (initially `true`). `HistoryRecorder` already owns effective recording policy and the store's paused state. A coherent interface therefore needs one app-owned source for inspector visibility and status presentation, without moving Utility-specific restore, snapshots, editing, or execution into a generic workspace API.

## Usage (caller's view)

The main window constructs one `InspectorVisibility` and passes it to concrete workspaces. The shell draws one selected-Utility heading and its History action; the active workspace consumes the same visibility state to place its existing History inspector. UI state is never interpreted as recording policy.

```rust
// workbench.rs: one instance for this application session
let inspector = Rc::new(InspectorVisibility::load(
    InspectorVisibilityPreferences::application_support().ok(),
));
let json = cx.new(|cx| JsonWorkspace::new(
    window, cx, clipboard.clone(), history.clone(), inspector.clone(),
));
// Registry construction is still the sole type-erasure boundary.
let view = construct(window, cx, clipboard.clone(), history.clone(), inspector.clone());
```

```rust
// workbench.rs: shell header, selected Utility is a typed UtilityId
let visible = self.inspector.is_visible(self.selected);
let status = RecordingStatus::for_utility(&self.history, self.selected);
let history_action = Button::new("workbench.history")
    .icon(IconName::PanelRight)
    .ghost()
    .tooltip(if visible { "Hide History" } else { "Show History" })
    .accessibility_label(if visible { "Hide History" } else { "Show History" })
    .on_click(cx.listener(|this, _, _, cx| {
        this.inspector.set_visible(this.selected, !this.inspector.is_visible(this.selected), cx);
    }));
// Header text is the actual status, e.g. "Recording off globally" or "Recording paused".
```

```rust
// json_workspace.rs: retain Utility-owned rows, restore confirmation and snapshot decoding
if self.inspector.is_visible(UtilityId::Json) {
    workspace = workspace.child(self.render_history(cx));
}
// The constructor subscribes to UtilityId::Json visibility changes and calls cx.notify().
// It keeps no second history_visible bool.
```

The same shell action works for every Utility, including type-erased registry workspaces, because all sessions share one visibility controller. Each workspace only reads its own typed ID; no generic execute or restore interface is added.

## Shape

### Types and signatures

```rust
// app/inspector_visibility.rs (new app-only module)
pub struct InspectorVisibility {
    visible: RefCell<HashSet<UtilityId>>, // absence means closed
    preferences: Option<InspectorVisibilityPreferences>,
    subscribers: Rc<RefCell<Subscribers>>, // same scoped callback pattern as HistoryRecorder
}

impl InspectorVisibility {
    pub fn load(preferences: Option<InspectorVisibilityPreferences>) -> Self;
    pub fn is_visible(&self, utility: UtilityId) -> bool;
    pub fn set_visible(&self, utility: UtilityId, visible: bool, cx: &mut App);
    pub fn subscribe(&self, utility: UtilityId,
        callback: impl Fn(&mut App) + 'static) -> InspectorSubscription;
}

// app/preferences.rs: layout-only file, e.g. inspector-visibility.v1.json
pub struct InspectorVisibilityPreferences { root: PathBuf }
impl InspectorVisibilityPreferences {
    pub fn application_support() -> Result<Self, PreferenceError>;
    pub fn load(&self) -> HashSet<UtilityId>; // parse slugs at the file boundary
    pub fn save(&self, visible: &HashSet<UtilityId>) -> Result<(), PreferenceError>;
}

// app/history.rs: presentation value derived from the existing authority
pub enum RecordingStatus {
    Enabled,
    OffGlobally,
    OffForUtility,
    Paused,
}
impl RecordingStatus {
    pub fn for_utility(recorder: &HistoryRecorder, utility: UtilityId) -> Self;
    pub fn label(self) -> &'static str;
}

// app/registry.rs: extend only the existing construction boundary
pub type WorkspaceConstructor = fn(
    &mut Window, &mut Context<Workbench>, Rc<dyn Clipboard>,
    Rc<HistoryRecorder>, Rc<InspectorVisibility>,
) -> AnyView;
```

`set_visible` is idempotent: an unchanged value performs no write or notification. A changed value updates memory, saves the layout preference atomically, then notifies the selected workspace subscriber and Workbench. Save failure is surfaced as a small non-payload UI warning while the in-memory choice remains effective for this launch. The file stores only Utility slugs and visibility booleans. It contains no History entry or editor content. Unknown slugs are ignored at load; an absent or invalid file yields all closed. The persisted set belongs to one controller, preventing lost updates from Workbench and Utility sessions writing separate copies.

`RecordingStatus::for_utility` reads `HistoryPolicy` plus `store().is_paused(id.slug())`; paused takes precedence when policy is enabled. It does not create a second recording flag. `HistoryRecorder::set_global_enabled` and `set_utility_enabled` should emit status notifications on change (take `&mut App` or provide one centralized mutation method), so Workbench and Settings redraw across windows. Recording defaults, JWT opt-out, retained-entry access, and pause semantics remain exactly as today. Hiding History does not call either setter.

### Module map and data flow

| Owner | Change |
| --- | --- |
| `workbench.rs` | Compact 228-ish px catalog and full-width row button, one Utility heading, icon and tooltip actions; use Registry metadata, preserve arrow-key focus and retained entities. Appearance controls move to Settings. Show status from `RecordingStatus`; toggle visibility through controller. |
| `launcher.rs` | Match compact catalog row treatment while preserving nonactivating panel, search, keyboard selection and focus. |
| `settings.rs` | Add System/Light/Dark segmented choice; invoke Workbench's existing appearance application through a small `configure_appearance(mode, cx)` method using `appearance::apply(mode, None, cx)`, which already refreshes all windows. Keep shortcut, per-Utility recording overrides, clear confirmations and build details in this window. |
| `preferences.rs` | Keep current workspace theme/favorites/recents schema. Add separate inspector visibility file so Workbench's frequent recents/theme saves cannot overwrite pane choices. |
| `history.rs` | Derive effective status and broadcast policy changes; no changes to storage, retention, snapshots, deletion or recording decisions. |
| `ui.rs` | Extend app-owned `panel` composition to accept a header action slot (`panel_with_actions` or `PanelHeader`), while GPUI Kit `Button`, `IconName`, tooltips and editor retain interaction ownership. Existing `panel` can call the new composition with no actions during migration. |
| `json_workspace.rs` | Keep `TabBar` mode; use kit `SelectState`/`Select` for two/four-space indentation and `Checkbox` for Sort keys. Move Paste/Clear to input header and Copy to result header. Keep live session and multiline adapter. |
| `utilities/random_string.rs` | Remove duplicate local title and `flex_1` from Custom field. Keep typed `NumberInput`; replace character-class button toggles with kit `Checkbox` controlled values. Keep Generate adjacent to configuration; place Copy All/Clear in results header and let results alone flex. Persist only controls, never generated values. |
| `text_diff/mod.rs` | Put Paste/Copy in each source pane header, preserve `WebDiffSurface` and grapheme adapter. Replace fixed `h_40()` source row with kit `v_resizable` panels and `ResizableState`; use kit `h_resizable` for original/updated where width permits. Keep mode `TabBar`. |
| Other Utility workspaces | Drop local `history_visible`, subscribe to the shared controller, remove misleading `History: on/off` labels, retain owned inspector/restore code. Migrate pane actions and typed controls where applicable using the same visual grammar, without a universal workspace renderer. |

The kit APIs exist in locked 0.6.6: `Button::icon`, `.ghost`, `.tooltip`, `.accessibility_label`; `Checkbox::checked` with controlled `on_change`; `TabBar::segmented`; `SelectState`/`Select`; `h_resizable`, `v_resizable`, `resizable_panel().size_range(...)`, and `ResizablePanelGroup::with_state`. Icons including `panel-right`, `star`, `copy`, `clipboard-paste`, `settings`, `search` ship in `gpui-kit-assets`. The `Select` delegate and resize state should be wired from their pinned signatures, not a custom popup or drag handler.

### Layout and focus

Treat width as a layout decision in the owning view, not a persisted breakpoint: normal width shows a resizable History pane beside the Utility; below a measured threshold, the explicit History action opens the same Utility-owned inspector in a kit-owned dialog/sheet with a close action. Store only open/closed intent, not the responsive presentation mode. Capture the previously focused editor handle before opening and restore it after dismissal. Give text editors and the WebView positive minimum dimensions; when the catalog plus two editors cannot fit, stack the editors. Resizable state is session-only until native drag/focus behavior is verified; visibility remains the sole new persisted layout datum.

## Synthesis decision

Candidate output for arena synthesis. This proposal keeps a single shell-owned visibility controller and Utility-owned pane content; the orchestrator will compare it with the independent candidate before selecting the implementation shape.

## Tradeoffs accepted

- We accept a constructor-parameter migration across 15 Utilities in exchange for a single typed visibility state that can be controlled from the shell without type-erased downcasts.
- We accept a separate small preference file in exchange for one writer and no collision with Workbench's existing theme/recents saves.
- We accept a transitional `panel` overload in `ui.rs` in exchange for moving actions next to content without cloning GPUI Kit controls or rewriting every Utility simultaneously.
- We accept session-only split sizes in exchange for a smaller persistence contract and less risk to text focus, selection and the embedded WebView.

## Alternatives considered

1. **Launcher-first, hidden catalog.** It gives editors more width, but makes discovery and category scanning depend on recall. It also leaves inspector policy, action placement and generator layout to every caller. Its smaller shell hides less complexity and changes the approved keyboard-first Workbench model.
2. **One generic workspace view with a `WorkspaceChrome`/`execute` trait.** It could centralize all headers and panes, but would expose every Utility's commands, result shape and restoration semantics through a wide erased interface. That crosses the source-defined Utility and registry boundaries for a visual redesign.
3. **Per-Utility persisted booleans.** It limits constructor edits, but leaves the shell unable to toggle a type-erased workspace and creates two visibility authorities or repeated disk reads. Interface depth is worse because callers must coordinate local state and the header.

## Open questions and risks

- Does native GPUI Kit rendering keep `ResizableState` and `WebDiffSurface` stable through a vertical drag and appearance change? Verify in the actual bundle before persisting sizes.
- Which narrow width keeps catalog, editor minimums, and History readable at 1000x700? Choose from native measurements; the HTML mock's container breakpoint is not a macOS metric.
- Does the Settings appearance action preserve focus across both windows when `appearance::apply(mode, None, cx)` refreshes them? Test System transitions and explicit modes.
- Are icon-only row actions announced and keyboard reachable in the native accessibility tree? Confirm after implementation rather than inferring from view code.

## Next implementation step

Build the typed visibility controller and status derivation, then wire the shell header and JSON workspace as the first vertical slice before carrying the same contract through the registry workspaces.

## Verification plan

Run focused preference/controller and recording-status tests, plus the relevant root gate (`make verify`; `make verify-full` if renderer/bundle sources change). Exercise native macOS layouts at 1000x700 and 1280x800 in System, Light and Dark, with History open/closed, long catalog names, keyboard-only use, and retained Utility sessions. Verify hiding History never changes `is_recording`, global override text is accurate, existing restore/delete/copy confirmations work, and Text Diff resizing preserves embedded renderer and extended-grapheme editing. Build/test results alone are not native acceptance.
