//! Text Diff's application-owned workspace boundary.
//!
//! The workspace exposes GPUI controls and never exposes WebKit/Wry types. The
//! macOS renderer is a child view of the existing GPUI window and receives only
//! an exact UTF-8 snapshot plus a monotonically increasing revision. A completed
//! current comparison is recorded once in fresh Rust History and restores the
//! exact old/new texts without rerunning the renderer.

mod renderer;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, Context, Entity, IntoElement, Render, Subscription, Task, Window};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    input::{InputEvent, TextareaState},
    list::ListState,
    resizable::{h_resizable, resizable_panel, v_resizable, ResizableState},
    tab::{Tab, TabBar},
    ActiveTheme as _, Disableable as _, Icon,
};
use sofdevtool_core::utilities::text_diff::{
    TextDiff, TextDiffMode, TextDiffRequest, TextDiffSnapshot,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::registry::UtilityId;
use crate::ui;
use crate::workspace_layout::{HistoryPlacement, WorkspaceLayout};
use renderer::{RendererStatus, TextDiffRenderer, WebDiffSurface};

pub const TEXT_DIFF_UTILITY_ID: &str = "text-diff";
const HISTORY_SETTLE_DELAY: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayMode {
    Split,
    Unified,
}

impl DisplayMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Unified => "unified",
        }
    }

    fn core(self) -> TextDiffMode {
        match self {
            Self::Split => TextDiffMode::Split,
            Self::Unified => TextDiffMode::Unified,
        }
    }

    fn from_core(mode: TextDiffMode) -> Self {
        match mode {
            TextDiffMode::Split => Self::Split,
            TextDiffMode::Unified => Self::Unified,
        }
    }
}

/// An in-memory Text Diff Utility Workspace Session.
pub struct TextDiffWorkspace {
    old: Entity<TextareaState>,
    new: Entity<TextareaState>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    renderer: TextDiffRenderer,
    mode: DisplayMode,
    revision: u64,
    recorded_revision: u64,
    settled_revision: Option<u64>,
    settle_task: Option<Task<()>>,
    suppress_render: bool,
    diagnostic: Option<String>,
    renderer_status: RendererStatus,
    renderer_status_changed: Rc<Cell<bool>>,
    copied: Option<usize>,
    history_view: HistoryViewState,
    layout: Entity<WorkspaceLayout>,
    _layout_subscription: Subscription,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    source_split: Entity<ResizableState>,
    workspace_split: Entity<ResizableState>,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl TextDiffWorkspace {
    #[cfg(test)]
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let layout = cx.new(|_| WorkspaceLayout::load(None));
        Self::new_with_layout(window, cx, clipboard, history, layout)
    }

    pub fn new_with_layout(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
        layout: Entity<WorkspaceLayout>,
    ) -> Self {
        let layout_subscription = cx.observe(&layout, |_, _, cx| cx.notify());
        let old = cx.new(|cx| TextareaState::new(window, cx));
        let new = cx.new(|cx| TextareaState::new(window, cx));
        let source_split = cx.new(|_| ResizableState::default());
        let workspace_split = cx.new(|_| ResizableState::default());
        let renderer_status_changed = Rc::new(Cell::new(false));
        let renderer = TextDiffRenderer::new(window, renderer_status_changed.clone(), cx);
        let diagnostic = renderer
            .render(
                1,
                old.read(cx).value().to_string(),
                new.read(cx).value().to_string(),
                DisplayMode::Split.as_str(),
            )
            .err();
        let renderer_status = renderer.status();
        let subscriptions = vec![
            cx.subscribe_in(
                &old,
                window,
                |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.edit_snapshot(window, cx)
                    }
                },
            ),
            cx.subscribe_in(
                &new,
                window,
                |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.edit_snapshot(window, cx)
                    }
                },
            ),
        ];
        let history_view = HistoryViewState::load(&history, TextDiff::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(TextDiff::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        let weak = cx.weak_entity();
        let history_list = ui::history_state(
            window,
            cx,
            "No retained comparisons yet.",
            Rc::new(move |id, _window, cx| {
                weak.update(cx, |this, cx| {
                    if this.history_view.select(id) {
                        cx.notify();
                    }
                })
                .ok();
            }),
        );
        let workspace = Self {
            old,
            new,
            clipboard,
            history,
            renderer,
            mode: DisplayMode::Split,
            revision: 1,
            // The initial empty comparison only initializes the renderer.
            recorded_revision: 1,
            settled_revision: None,
            settle_task: None,
            suppress_render: false,
            diagnostic,
            renderer_status,
            renderer_status_changed,
            copied: None,
            history_view,
            layout,
            _layout_subscription: layout_subscription,
            history_list,
            source_split,
            workspace_split,
            _history_subscription: history_subscription,
            _subscriptions: subscriptions,
        };
        workspace.sync_history(cx);
        workspace.observe_renderer_status(cx);
        workspace
    }

    /// The workbench calls this when this workspace is selected or covered.
    /// A native child view is outside GPUI's normal clipping tree, so hiding it
    /// explicitly prevents it from overlaying another workspace.
    ///
    /// Appearance is pushed from [`Render::render`], which only runs while this
    /// workspace is the active view, so an activation or a workbench appearance
    /// change (`appearance::apply` refreshes every window) both reach the
    /// renderer. `set_active` keeps the native child's visibility in sync.
    pub fn set_active(&self, active: bool) {
        self.renderer.set_active(active);
    }

    /// Return keyboard ownership from the native child to GPUI controls.
    pub fn focus_parent(&self) {
        self.renderer.focus_parent();
    }

    /// Standalone-proof hook: this submits a malformed request through the
    /// same local bridge, so the visible diagnostic proves IPC error handling.
    #[cfg(debug_assertions)]
    pub fn simulate_renderer_failure(&mut self, cx: &mut Context<Self>) {
        self.settle_task.take();
        self.settled_revision = None;
        self.revision += 1;
        self.diagnostic = self.renderer.render_proof_failure(self.revision).err();
        self.renderer_status = self.renderer.status();
        cx.notify();
    }

    fn render_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.suppress_render {
            return;
        }
        self.revision += 1;
        self.diagnostic = self
            .renderer
            .render(
                self.revision,
                self.old.read(cx).value().to_string(),
                self.new.read(cx).value().to_string(),
                self.mode.as_str(),
            )
            .err();
        self.renderer_status = self.renderer.status();
        self.record_if_ready(cx);
        window.refresh();
        cx.notify();
    }

    fn edit_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settle_task.take();
        self.settled_revision = None;
        self.render_snapshot(window, cx);
        let revision = self.revision;
        let executor = cx.background_executor().clone();
        self.settle_task = Some(cx.spawn(async move |this, cx| {
            executor.timer(HISTORY_SETTLE_DELAY).await;
            this.update(cx, |this, cx| {
                if this.revision == revision {
                    this.settled_revision = Some(revision);
                    this.record_if_ready(cx);
                }
            })
            .ok();
        }));
    }

    /// Records only the settled current comparison once the renderer is ready.
    fn record_if_ready(&mut self, cx: &mut Context<Self>) {
        if self.suppress_render
            || self.recorded_revision == self.revision
            || self.settled_revision != Some(self.revision)
        {
            return;
        }
        if !matches!(self.renderer_status, RendererStatus::Ready) {
            return;
        }
        let request = TextDiffRequest::new(
            self.old.read(cx).value().to_string(),
            self.new.read(cx).value().to_string(),
            self.mode.core(),
        );
        let evaluation = <TextDiff as Utility>::evaluate(&request);
        let Some(snapshot) = <TextDiff as Utility>::snapshot(&request, &evaluation) else {
            return;
        };
        self.recorded_revision = self.revision;
        let payload = serde_json::to_value(&snapshot).expect("a Text Diff snapshot serializes");
        let result = self
            .history
            .record(TextDiff::ID, TextDiff::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, TextDiff::ID, result);
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, TextDiff::ID);
        self.sync_history(cx);
        cx.notify();
    }

    /// IPC handlers run outside GPUI's entity update path. They set this
    /// signal; this lightweight observer transfers the resulting status into
    /// the workspace and invalidates the visible status immediately.
    fn observe_renderer_status(&self, cx: &mut Context<Self>) {
        let changed = self.renderer_status_changed.clone();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| loop {
            executor.timer(Duration::from_millis(25)).await;
            if this
                .update(cx, |this, cx| {
                    if changed.replace(false) {
                        this.renderer_status = this.renderer.status();
                        this.record_if_ready(cx);
                        cx.notify();
                    }
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    fn set_mode(&mut self, mode: DisplayMode, window: &mut Window, cx: &mut Context<Self>) {
        self.renderer.focus_parent();
        self.mode = mode;
        self.edit_snapshot(window, cx);
    }

    fn paste(&self, editor: &Entity<TextareaState>, window: &mut Window, cx: &mut Context<Self>) {
        self.renderer.focus_parent();
        if let Some(text) = self.clipboard.read_text(cx) {
            editor.update(cx, |state, cx| state.replace_all(text, window, cx));
        }
    }

    /// Reflects the owning view's History rows and selection into the kit list.
    fn sync_history(&self, cx: &mut Context<Self>) {
        ui::history_set_rows(&self.history_list, self.history_items(), cx);
        ui::history_set_selected(&self.history_list, self.history_view.selected.clone(), cx);
    }

    fn copy(&mut self, index: usize, text: String, cx: &mut Context<Self>) {
        self.clipboard.write_text(&text, cx);
        self.copied = Some(index);
        cx.notify();
    }

    fn request_restore(
        &mut self,
        entry: HistoryEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = decode_snapshot(&entry) else {
            self.history_view.error =
                Some("This History entry uses a snapshot version this build cannot read.".into());
            cx.notify();
            return;
        };
        let current_old = self.old.read(cx).value().to_string();
        let current_new = self.new.read(cx).value().to_string();
        let same = current_old == snapshot.old && current_new == snapshot.new;
        if (!current_old.is_empty() || !current_new.is_empty()) && !same {
            self.history_view.pending_restore = Some(entry);
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current non-empty Text Diff session.",
                "Restore",
                "Cancel",
                {
                    let weak = weak.clone();
                    move |window, cx| {
                        let _ = weak.update(cx, |this, cx| this.confirm_restore(window, cx));
                    }
                },
                {
                    let weak = weak.clone();
                    move |_window, cx| {
                        let _ = weak.update(cx, |this, cx| this.cancel_restore(cx));
                    }
                },
            );
            cx.notify();
        } else {
            self.apply_restore(snapshot, window, cx);
        }
    }

    fn apply_restore(
        &mut self,
        snapshot: TextDiffSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settle_task.take();
        self.settled_revision = None;
        self.suppress_render = true;
        self.old.update(cx, |state, cx| {
            state.set_value(snapshot.old.clone(), window, cx)
        });
        self.new.update(cx, |state, cx| {
            state.set_value(snapshot.new.clone(), window, cx)
        });
        self.mode = DisplayMode::from_core(snapshot.mode);
        self.suppress_render = false;
        // Mark the next render as already captured before it can report
        // Ready synchronously. A later IPC readiness callback sees the same
        // revision and cannot record this restore either.
        self.recorded_revision = self.revision + 1;
        self.render_snapshot(window, cx);
        self.history_view.pending_restore = None;
        self.copied = None;
        self.sync_history(cx);
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.history_view.pending_restore.clone() {
            if self
                .history_view
                .retained(&self.history, TextDiff::ID, &entry)
            {
                if let Some(snapshot) = decode_snapshot(&entry) {
                    self.apply_restore(snapshot, window, cx);
                }
            } else {
                self.reconcile_history(cx);
            }
        }
    }

    fn cancel_restore(&mut self, cx: &mut Context<Self>) {
        self.history_view.pending_restore = None;
        cx.notify();
    }

    fn restore_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected) = self.history_view.selected.clone() else {
            return;
        };
        if let Some(entry) = self
            .history_view
            .entries
            .iter()
            .find(|entry| entry.id == selected)
            .cloned()
        {
            if self
                .history_view
                .retained(&self.history, TextDiff::ID, &entry)
            {
                self.request_restore(entry, window, cx);
            } else {
                self.reconcile_history(cx);
            }
        }
    }

    fn history_items(&self) -> Vec<ui::HistoryRow> {
        self.history_view
            .entries
            .iter()
            .map(|entry| {
                let snapshot = decode_snapshot(entry);
                ui::HistoryRow {
                    id: entry.id.clone(),
                    label: entry.captured_at.clone(),
                    preview: snapshot
                        .as_ref()
                        .map(|snapshot| preview_line(&snapshot.new, &snapshot.old))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: snapshot
                        .is_none()
                        .then(|| "Unavailable snapshot".to_owned()),
                    selectable: true,
                }
            })
            .collect()
    }

    pub(crate) fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.history_view.selected.clone();
        let restore_enabled = selected
            .as_ref()
            .and_then(|id| {
                self.history_view
                    .entries
                    .iter()
                    .find(|entry| &entry.id == id)
            })
            .map(|entry| decode_snapshot(entry).is_some())
            .unwrap_or(false);
        let restore = Button::new("text-diff.history.restore-selected")
            .label("Restore selected")
            .disabled(!restore_enabled)
            .on_click(cx.listener(|this, _event, window, cx| this.restore_selected(window, cx)));
        let theme = cx.theme().clone();
        div()
            .flex()
            .flex_col()
            .w_64()
            .min_h_0()
            .p_3()
            .gap_2()
            .border_l_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .child(ui::history_panel(
                cx,
                &self.history_list,
                "History",
                format!("{}/25", self.history_view.entries.len()),
                restore,
            ))
    }
}

impl Drop for TextDiffWorkspace {
    fn drop(&mut self) {
        self.renderer.set_active(false);
    }
}

impl Render for TextDiffWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Pushed while this workspace is the active view; `appearance::apply`
        // refreshes every window, so both the first activation and later
        // Latte/Frappe or System changes reach the native child.
        self.renderer.set_appearance(cx);
        let mode = self.mode;
        let old = ui::multiline_editor(&self.old, false, "text-diff.original");
        let new = ui::multiline_editor(&self.new, false, "text-diff.updated");
        let theme = cx.theme().clone();
        let selected_index = if mode == DisplayMode::Split { 0 } else { 1 };
        let mode_control = TabBar::new("text-diff.mode")
            .segmented()
            .selected_index(selected_index)
            .children([Tab::new().label("Split"), Tab::new().label("Unified")])
            .on_click(cx.listener(|this, index, window, cx| {
                let mode = if *index == 0 {
                    DisplayMode::Split
                } else {
                    DisplayMode::Unified
                };
                this.set_mode(mode, window, cx);
            }));
        let original_actions = div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                Button::new("text-diff.paste-original")
                    .icon(Icon::new(IconName::ClipboardPaste))
                    .tooltip("Paste original text")
                    .accessibility_label("Paste original text")
                    .ghost()
                    .on_click(
                        cx.listener(|this, _event, window, cx| this.paste(&this.old, window, cx)),
                    ),
            )
            .child(
                Button::new("text-diff.copy-original")
                    .icon(Icon::new(IconName::Copy))
                    .tooltip("Copy original text")
                    .accessibility_label("Copy original text")
                    .ghost()
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        let text = this.old.read(cx).value().to_string();
                        this.copy(0, text, cx);
                    })),
            );
        let updated_actions = div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                Button::new("text-diff.paste-updated")
                    .icon(Icon::new(IconName::ClipboardPaste))
                    .tooltip("Paste updated text")
                    .accessibility_label("Paste updated text")
                    .ghost()
                    .on_click(
                        cx.listener(|this, _event, window, cx| this.paste(&this.new, window, cx)),
                    ),
            )
            .child(
                Button::new("text-diff.copy-updated")
                    .icon(Icon::new(IconName::Copy))
                    .tooltip("Copy updated text")
                    .accessibility_label("Copy updated text")
                    .ghost()
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        let text = this.new.read(cx).value().to_string();
                        this.copy(1, text, cx);
                    })),
            );
        let sources = h_resizable("text-diff.sources")
            .with_state(&self.source_split)
            .child(
                resizable_panel()
                    .size_range(gpui::px(220.)..gpui::px(2000.))
                    .child(ui::pane(cx, "Original", original_actions, old)),
            )
            .child(
                resizable_panel()
                    .size_range(gpui::px(220.)..gpui::px(2000.))
                    .child(ui::pane(cx, "Updated", updated_actions, new)),
            );
        let comparison_actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(mode_control)
            .child(ui::copy_feedback(
                cx,
                self.copied.is_some(),
                "Copied to Clipboard",
            ));
        let panels = v_resizable("text-diff.panels")
            .with_state(&self.workspace_split)
            .child(
                resizable_panel()
                    .size_range(gpui::px(120.)..gpui::px(800.))
                    .child(sources),
            )
            .child(
                resizable_panel()
                    .size_range(gpui::px(180.)..gpui::px(2000.))
                    .child(ui::pane(
                        cx,
                        "Comparison",
                        comparison_actions,
                        WebDiffSurface::new(self.renderer.clone()),
                    )),
            );
        let mut main = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(div().flex().flex_1().min_h_0().child(panels));
        main = match &self.renderer_status {
            RendererStatus::Loading => {
                main.child(div().text_xs().child("Loading local diff renderer…"))
            }
            RendererStatus::Ready => main,
            RendererStatus::Failure(message) => main.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Error,
                message,
                None,
            )),
        };
        if contains_complex_emoji(&self.old.read(cx).value())
            || contains_complex_emoji(&self.new.read(cx).value())
        {
            main = main.child(div().text_xs().child(
                "Complex emoji uses whole-line highlighting to preserve Unicode correctness.",
            ));
        }
        if let Some(message) = &self.diagnostic {
            main = main.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Error,
                message,
                None,
            ));
        }
        if let Some(error) = self.history_view.error.clone() {
            main = main.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Text Diff History: {error}"),
                None,
            ));
        }

        let mut root = div()
            .id("text-diff-workspace")
            .flex()
            .flex_row()
            .gap_3()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(main);
        if self.layout.read(cx).placement(UtilityId::TextDiff) == HistoryPlacement::Inline {
            root = root.child(self.render_history(cx));
        }
        root
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<TextDiffSnapshot> {
    if entry.snapshot_version != TextDiff::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

fn preview_line(new: &str, old: &str) -> String {
    let source = if new.trim().is_empty() { old } else { new };
    let single_line = source.replace('\n', " ");
    let trimmed = single_line.trim();
    let mut preview: String = trimmed.chars().take(80).collect();
    if trimmed.chars().count() > 80 {
        preview.push('…');
    }
    if preview.is_empty() {
        "Empty comparison".to_owned()
    } else {
        preview
    }
}

fn contains_complex_emoji(text: &str) -> bool {
    text.chars().any(|character| {
        let scalar = character as u32;
        (0x1F000..=0x1FAFF).contains(&scalar) || (0x2600..=0x27BF).contains(&scalar)
    })
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use gpui::{App, VisualTestContext};
    use gpui_kit::component::Root;

    use crate::history::{HistoryStore, SystemClock};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct TestClipboard(RefCell<Option<String>>);

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            self.0.borrow().clone()
        }

        fn write_text(&self, text: &str, _cx: &mut App) {
            *self.0.borrow_mut() = Some(text.to_owned());
        }
    }

    fn isolated_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-text-diff-redesign-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[gpui::test]
    fn mode_copy_and_exact_history_restore_use_live_controls(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let old = "let café = 👩🏽‍💻\n";
        let new = "let caffè = 🇮🇹 1️⃣\n";
        history
            .record(
                TextDiff::ID,
                TextDiff::SNAPSHOT_VERSION,
                serde_json::to_value(TextDiffSnapshot {
                    old: old.into(),
                    new: new.into(),
                    mode: TextDiffMode::Unified,
                })
                .unwrap(),
            )
            .unwrap();
        let entry = history.load(TextDiff::ID).unwrap().pop().unwrap();
        let clipboard = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| TextDiffWorkspace::new(window, cx, clipboard.clone(), history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.old.read(cx).value().to_string()),
            ""
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.new.read(cx).value().to_string()),
            ""
        );

        // The kit `TabBar` (segmented) drives the same workspace setter.
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.set_mode(DisplayMode::Unified, window, cx)
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            DisplayMode::Unified
        );
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.set_mode(DisplayMode::Split, window, cx));
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            DisplayMode::Split
        );

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.new.update(cx, |state, cx| {
                    state.replace_all("let café = \"family 👨‍👩‍👧‍👦\"\n", window, cx)
                })
            });
            window.draw(cx).clear(cx);
        });

        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| {
                let text = view.new.read(cx).value().to_string();
                view.copy(1, text, cx);
            });
        });
        assert_eq!(
            clipboard.0.borrow().as_deref(),
            Some("let café = \"family 👨‍👩‍👧‍👦\"\n")
        );
        let entries_before_restore = history.load(TextDiff::ID).unwrap().len();

        // Selecting a retained History row and then the explicit Restore
        // action requests confirmation through the kit dialog; confirming
        // applies the snapshot.
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                assert!(view.history_view.select(&entry.id));
                view.sync_history(cx);
                view.restore_selected(window, cx);
                assert!(view.history_view.pending_restore.is_some());
                view.confirm_restore(window, cx);
            });
            window.draw(cx).clear(cx);
        });
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.old.read(cx).value().to_string()),
            old
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.new.read(cx).value().to_string()),
            new
        );
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            DisplayMode::Unified
        );
        assert_eq!(
            history.load(TextDiff::ID).unwrap().len(),
            entries_before_restore
        );
        cx.executor().advance_clock(HISTORY_SETTLE_DELAY);
        cx.run_until_parked();
        assert_eq!(
            history.load(TextDiff::ID).unwrap().len(),
            entries_before_restore,
            "restoring must cancel the pending comparison recording"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn fresh_session_records_only_latest_settled_valid_comparison(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let clipboard = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| TextDiffWorkspace::new(window, cx, clipboard, history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.old.read(cx).value().to_string()),
            ""
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.new.read(cx).value().to_string()),
            ""
        );
        assert!(history.load(TextDiff::ID).unwrap().is_empty());

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.old
                    .update(cx, |state, cx| state.replace_all("café", window, cx));
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.old
                    .update(cx, |state, cx| state.replace_all("caffè 👩🏽‍💻", window, cx));
                view.new
                    .update(cx, |state, cx| state.replace_all("caffè 🇮🇹", window, cx));
                view.renderer_status = RendererStatus::Ready;
                view.record_if_ready(cx);
            });
            window.draw(cx).clear(cx);
        });
        assert!(history.load(TextDiff::ID).unwrap().is_empty());

        cx.executor().advance_clock(Duration::from_millis(199));
        cx.run_until_parked();
        assert!(history.load(TextDiff::ID).unwrap().is_empty());
        cx.executor().advance_clock(Duration::from_millis(1));
        cx.run_until_parked();
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| {
                // Headless GPUI does not attach the native child WebView. Feed
                // the readiness boundary after the real editor/debounce path.
                view.renderer_status = RendererStatus::Ready;
                view.record_if_ready(cx);
                view.record_if_ready(cx);
            });
        });
        let entries = history.load(TextDiff::ID).unwrap();
        assert_eq!(entries.len(), 1);
        let snapshot = decode_snapshot(&entries[0]).unwrap();
        assert_eq!(snapshot.old, "caffè 👩🏽‍💻");
        assert_eq!(snapshot.new, "caffè 🇮🇹");
        assert_eq!(snapshot.mode, TextDiffMode::Split);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.old
                    .update(cx, |state, cx| state.replace_all("", window, cx));
                view.new
                    .update(cx, |state, cx| state.replace_all("", window, cx));
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(HISTORY_SETTLE_DELAY);
        cx.run_until_parked();
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| {
                view.renderer_status = RendererStatus::Ready;
                view.record_if_ready(cx);
            });
        });
        assert_eq!(history.load(TextDiff::ID).unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
