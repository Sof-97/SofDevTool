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
use gpui::{div, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::utilities::text_diff::{
    TextDiff, TextDiffMode, TextDiffRequest, TextDiffSnapshot,
};
use sofdevtool_core::utility::Utility;
use sofui::{
    copy_feedback, diagnostic_banner, panel, view_click, Button, ConfirmationBar,
    DiagnosticSeverity, SegmentedControl, SegmentedControlFocus, SegmentedOption, SelectableList,
    SelectableListFocus, SelectableRow, TextEditor, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use renderer::{RendererStatus, TextDiffRenderer, WebDiffSurface};

pub const TEXT_DIFF_UTILITY_ID: &str = "text-diff";

struct ButtonFocus {
    paste_original: FocusHandle,
    paste_updated: FocusHandle,
    copy_original: FocusHandle,
    copy_updated: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

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
    old: TextEditor,
    new: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    renderer: TextDiffRenderer,
    mode: DisplayMode,
    revision: u64,
    recorded_revision: u64,
    suppress_render: bool,
    diagnostic: Option<String>,
    renderer_status: RendererStatus,
    renderer_status_changed: Rc<Cell<bool>>,
    copied: Option<usize>,
    history_view: HistoryViewState,
    history_visible: bool,
    history_focus: SelectableListFocus,
    choice_focus: SegmentedControlFocus,
    _history_subscription: HistorySubscription,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl TextDiffWorkspace {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let old = TextEditor::new(window, cx);
        let new = TextEditor::new(window, cx);
        old.assign_text("let café = \"👨‍👩‍👧‍👦\"\nlet flag = \"🏳️‍🌈\"\n", window, cx);
        new.assign_text(
            "let café = \"family 👨‍👩‍👧‍👦\"\nlet flag = \"🏳️‍🌈\"\nlet ready = true\n",
            window,
            cx,
        );
        let renderer_status_changed = Rc::new(Cell::new(false));
        let renderer = TextDiffRenderer::new(window, renderer_status_changed.clone());
        let diagnostic = renderer
            .render(1, old.text(cx), new.text(cx), DisplayMode::Split.as_str())
            .err();
        let renderer_status = renderer.status();
        let subscriptions = vec![
            old.on_change_in(window, cx, |this, window, cx| {
                this.render_snapshot(window, cx)
            }),
            new.on_change_in(window, cx, |this, window, cx| {
                this.render_snapshot(window, cx)
            }),
        ];
        let history_view = HistoryViewState::load(&history, TextDiff::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(TextDiff::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        let workspace = Self {
            old,
            new,
            clipboard,
            history,
            renderer,
            mode: DisplayMode::Split,
            revision: 1,
            // The initial sample is not a user operation, so it is not recorded.
            recorded_revision: 1,
            suppress_render: false,
            diagnostic,
            renderer_status,
            renderer_status_changed,
            copied: None,
            history_view,
            history_visible: true,
            history_focus: SelectableListFocus::new(),
            choice_focus: SegmentedControlFocus::new(),
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                paste_original: cx.focus_handle().tab_stop(true).tab_index(0),
                paste_updated: cx.focus_handle().tab_stop(true).tab_index(0),
                copy_original: cx.focus_handle().tab_stop(true).tab_index(0),
                copy_updated: cx.focus_handle().tab_stop(true).tab_index(0),
                history_toggle: cx.focus_handle().tab_stop(true).tab_index(0),
                history_restore: cx.focus_handle().tab_stop(true).tab_index(0),
                history_confirm: cx.focus_handle().tab_stop(true).tab_index(0),
                history_cancel: cx.focus_handle().tab_stop(true).tab_index(0),
            },
            _subscriptions: subscriptions,
        };
        workspace.observe_renderer_status(cx);
        workspace
    }

    /// The workbench calls this when this workspace is selected or covered.
    /// A native child view is outside GPUI's normal clipping tree, so hiding it
    /// explicitly prevents it from overlaying another workspace.
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
                self.old.text(cx),
                self.new.text(cx),
                self.mode.as_str(),
            )
            .err();
        self.renderer_status = self.renderer.status();
        self.record_if_ready(cx);
        window.refresh();
        cx.notify();
    }

    /// Records one completed current comparison once the renderer is ready.
    fn record_if_ready(&mut self, cx: &mut Context<Self>) {
        if self.suppress_render || self.recorded_revision == self.revision {
            return;
        }
        if !matches!(self.renderer_status, RendererStatus::Ready) {
            return;
        }
        let request = TextDiffRequest::new(self.old.text(cx), self.new.text(cx), self.mode.core());
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
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, TextDiff::ID);
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
        self.render_snapshot(window, cx);
    }

    fn paste(&self, editor: &TextEditor, window: &mut Window, cx: &mut Context<Self>) {
        self.renderer.focus_parent();
        if let Some(text) = self.clipboard.read_text(cx) {
            editor.edit_text(text, window, cx);
        }
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
        let current_old = self.old.text(cx);
        let current_new = self.new.text(cx);
        let same = current_old == snapshot.old && current_new == snapshot.new;
        if (!current_old.is_empty() || !current_new.is_empty()) && !same {
            self.history_view.pending_restore = Some(entry);
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
        self.suppress_render = true;
        self.old.assign_text(snapshot.old.clone(), window, cx);
        self.new.assign_text(snapshot.new.clone(), window, cx);
        self.mode = DisplayMode::from_core(snapshot.mode);
        self.suppress_render = false;
        // Mark the next render as already captured before it can report
        // Ready synchronously. A later IPC readiness callback sees the same
        // revision and cannot record this restore either.
        self.recorded_revision = self.revision + 1;
        self.render_snapshot(window, cx);
        self.history_view.pending_restore = None;
        self.copied = None;
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

    fn history_items(&self) -> Vec<SelectableRow> {
        self.history_view
            .entries
            .iter()
            .map(|entry| {
                let snapshot = decode_snapshot(entry);
                SelectableRow {
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

    fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
        let actions = div().flex().flex_row().gap_2().child(
            Button::with_id("text-diff.history.restore-selected", "Restore selected")
                .disabled(!restore_enabled)
                .focus_handle(self.focus.history_restore.clone())
                .on_click(view_click(cx, |this, window, cx| {
                    this.restore_selected(window, cx);
                })),
        );
        let weak = cx.weak_entity();
        let panel = SelectableList::new(
            "text-diff.history",
            "History",
            self.history_items(),
            selected,
            "No retained comparisons yet.",
            self.history_focus.clone(),
        )
        .summary(format!("{}/25", self.history_view.entries.len()))
        .on_select(Rc::new(move |id, _window, cx| {
            weak.update(cx, |this, cx| {
                if this.history_view.select(id) {
                    cx.notify();
                }
            })
            .ok();
        }))
        .actions(actions);
        div()
            .flex()
            .flex_col()
            .w_64()
            .min_h_0()
            .p_3()
            .gap_2()
            .border_l_1()
            .border_color(ThemeTokens::active().border())
            .bg(ThemeTokens::active().surface())
            .child(panel)
    }

    fn render_restore_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        ConfirmationBar::new(
            "text-diff.history.restore",
            "Restoring this History entry replaces the current non-empty Text Diff session.",
            "Restore",
            "Cancel",
        )
        .focus_handles(
            self.focus.history_confirm.clone(),
            self.focus.history_cancel.clone(),
        )
        .on_confirm(view_click(cx, |this, window, cx| {
            this.confirm_restore(window, cx)
        }))
        .on_cancel(view_click(cx, |this, _window, cx| this.cancel_restore(cx)))
    }
}

impl Drop for TextDiffWorkspace {
    fn drop(&mut self) {
        self.renderer.set_active(false);
    }
}

impl Render for TextDiffWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = self.mode;
        let old = self.old.render(false, "text-diff.original");
        let new = self.new.render(false, "text-diff.updated");
        let tokens = ThemeTokens::active();
        let mode_control = SegmentedControl::new(
            "text-diff.mode",
            "Comparison layout",
            vec![
                SegmentedOption::new("split", "Split"),
                SegmentedOption::new("unified", "Unified"),
            ],
            Some(mode.as_str().to_owned()),
            self.choice_focus.clone(),
        )
        .on_change(Rc::new({
            let weak = cx.weak_entity();
            move |id, window, cx| {
                let mode = match id {
                    "split" => DisplayMode::Split,
                    "unified" => DisplayMode::Unified,
                    _ => return,
                };
                weak.update(cx, |this, cx| this.set_mode(mode, window, cx))
                    .ok();
            }
        }));
        let mut main = div()
            .flex()
            .flex_col()
            .gap_3()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .bg(tokens.background())
            .text_color(tokens.text())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(mode_control)
                    .child(
                        Button::with_id("text-diff.paste-original", "Paste original")
                            .focus_handle(self.focus.paste_original.clone())
                            .on_click(view_click(cx, |this, window, cx| {
                                let old = &this.old;
                                this.paste(old, window, cx)
                            })),
                    )
                    .child(
                        Button::with_id("text-diff.paste-updated", "Paste updated")
                            .focus_handle(self.focus.paste_updated.clone())
                            .on_click(view_click(cx, |this, window, cx| {
                                let new = &this.new;
                                this.paste(new, window, cx)
                            })),
                    )
                    .child(div().flex_1())
                    .child(copy_feedback(self.copied.is_some(), "Copied to Clipboard"))
                    .child(
                        Button::with_id(
                            "text-diff.history.toggle",
                            if self.history_visible {
                                "History: on"
                            } else {
                                "History: off"
                            },
                        )
                        .focus_handle(self.focus.history_toggle.clone())
                        .on_click(view_click(cx, |this, _window, cx| {
                            this.history_visible = !this.history_visible;
                            cx.notify();
                        })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::with_id("text-diff.copy-original", "Copy original")
                            .focus_handle(self.focus.copy_original.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                let text = this.old.text(cx);
                                this.copy(0, text, cx);
                            })),
                    )
                    .child(
                        Button::with_id("text-diff.copy-updated", "Copy updated")
                            .focus_handle(self.focus.copy_updated.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                let text = this.new.text(cx);
                                this.copy(1, text, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .h_40()
                    .flex_shrink_0()
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(panel("Original", "editable", old)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(panel("Updated", "editable", new)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .overflow_hidden()
                    .child(panel(
                        "Comparison",
                        "live, selectable preview",
                        WebDiffSurface::new(self.renderer.clone()),
                    )),
            );
        if self.history_view.pending_restore.is_some() {
            main = main.child(self.render_restore_confirmation(cx));
        }
        main = match &self.renderer_status {
            RendererStatus::Loading => {
                main.child(div().text_xs().child("Loading local diff renderer…"))
            }
            RendererStatus::Ready => main,
            RendererStatus::Failure(message) => {
                main.child(diagnostic_banner(DiagnosticSeverity::Error, message, None))
            }
        };
        if contains_complex_emoji(&self.old.text(cx)) || contains_complex_emoji(&self.new.text(cx))
        {
            main = main.child(div().text_xs().child(
                "Complex emoji uses whole-line highlighting to preserve Unicode correctness.",
            ));
        }
        if let Some(message) = &self.diagnostic {
            main = main.child(diagnostic_banner(DiagnosticSeverity::Error, message, None));
        }
        if let Some(error) = self.history_view.error.clone() {
            main = main.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
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
        if self.history_visible {
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

    use gpui::{App, Entity, VisualTestContext};

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

    struct TestRoot(Entity<TextDiffWorkspace>);

    impl Render for TestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
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
        cx.update(sofui::init);
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
            TestRoot(view)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).choice_focus.clone();
            window.focus(&focus.handle("text-diff.mode", "unified", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            DisplayMode::Unified
        );
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).choice_focus.clone();
            window.focus(&focus.handle("text-diff.mode", "split", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            DisplayMode::Split
        );

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.copy_updated.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            clipboard.0.borrow().as_deref(),
            Some("let café = \"family 👨‍👩‍👧‍👦\"\nlet flag = \"🏳️‍🌈\"\nlet ready = true\n")
        );
        let entries_before_restore = history.load(TextDiff::ID).unwrap().len();

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).history_focus.clone();
            window.focus(&focus.handle(&entry.id, cx), cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.history_restore.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert!(workspace.read_with(&cx, |view, _| view.history_view.pending_restore.is_some()));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.history_confirm.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(workspace.read_with(&cx, |view, cx| view.old.text(cx)), old);
        assert_eq!(workspace.read_with(&cx, |view, cx| view.new.text(cx)), new);
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            DisplayMode::Unified
        );
        assert_eq!(
            history.load(TextDiff::ID).unwrap().len(),
            entries_before_restore
        );
        fs::remove_dir_all(root).unwrap();
    }
}
