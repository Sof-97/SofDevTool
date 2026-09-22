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
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, panel, view_click, Button, ButtonVariant, DiagnosticSeverity,
    HistoryItem, HistoryPanel, LabeledField, TextEditor, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder};
use renderer::{RendererStatus, TextDiffRenderer, WebDiffSurface};

pub const TEXT_DIFF_UTILITY_ID: &str = "text-diff";

struct ButtonFocus {
    split: FocusHandle,
    unified: FocusHandle,
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
    history_entries: Vec<HistoryEntry>,
    history_selected: Option<String>,
    history_visible: bool,
    history_error: Option<String>,
    pending_restore: Option<HistoryEntry>,
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
        old.set_text("let café = \"👨‍👩‍👧‍👦\"\nlet flag = \"🏳️‍🌈\"\n", window, cx);
        new.set_text(
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
        let (history_entries, history_error) = match history.load(TextDiff::ID) {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
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
            history_entries,
            history_selected: None,
            history_visible: true,
            history_error,
            pending_restore: None,
            focus: ButtonFocus {
                split: cx.focus_handle().tab_stop(true).tab_index(0),
                unified: cx.focus_handle().tab_stop(true).tab_index(0),
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
        match self
            .history
            .record(TextDiff::ID, TextDiff::SNAPSHOT_VERSION, payload)
        {
            Ok(entries) => {
                self.history_entries = entries;
                self.history_error = None;
            }
            Err(error) => {
                self.history_error = Some(error.to_string());
                cx.notify();
            }
        }
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
            editor.replace_all(text, window, cx);
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
            self.history_error =
                Some("This History entry uses a snapshot version this build cannot read.".into());
            cx.notify();
            return;
        };
        let current_old = self.old.text(cx);
        let current_new = self.new.text(cx);
        let same = current_old == snapshot.old && current_new == snapshot.new;
        if (!current_old.is_empty() || !current_new.is_empty()) && !same {
            self.pending_restore = Some(entry);
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
        self.old.set_text(snapshot.old.clone(), window, cx);
        self.new.set_text(snapshot.new.clone(), window, cx);
        self.mode = DisplayMode::from_core(snapshot.mode);
        self.suppress_render = false;
        self.render_snapshot(window, cx);
        // Restoring never records a new operation.
        self.recorded_revision = self.revision;
        self.pending_restore = None;
        self.copied = None;
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.pending_restore.clone() {
            if let Some(snapshot) = decode_snapshot(&entry) {
                self.apply_restore(snapshot, window, cx);
            }
        }
    }

    fn cancel_restore(&mut self, cx: &mut Context<Self>) {
        self.pending_restore = None;
        cx.notify();
    }

    fn restore_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected) = self.history_selected.clone() else {
            return;
        };
        if let Some(entry) = self
            .history_entries
            .iter()
            .find(|entry| entry.id == selected)
            .cloned()
        {
            self.request_restore(entry, window, cx);
        }
    }

    fn history_items(&self) -> Vec<HistoryItem> {
        self.history_entries
            .iter()
            .map(|entry| {
                let snapshot = decode_snapshot(entry);
                HistoryItem {
                    id: entry.id.clone(),
                    label: entry.captured_at.clone(),
                    preview: snapshot
                        .as_ref()
                        .map(|snapshot| preview_line(&snapshot.new, &snapshot.old))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.history_selected.clone();
        let restore_enabled = selected
            .as_ref()
            .and_then(|id| self.history_entries.iter().find(|entry| &entry.id == id))
            .map(|entry| decode_snapshot(entry).is_some())
            .unwrap_or(false);
        let actions = div().flex().flex_row().gap_2().child(
            Button::new("Restore selected")
                .disabled(!restore_enabled)
                .focus_handle(self.focus.history_restore.clone())
                .on_click(view_click(cx, |this, window, cx| {
                    this.restore_selected(window, cx);
                })),
        );
        let weak = cx.weak_entity();
        let panel = HistoryPanel::new(
            self.history_items(),
            selected,
            "No retained comparisons yet.",
        )
        .on_select(Rc::new(move |id, _window, cx| {
            weak.update(cx, |this, cx| {
                this.history_selected = Some(id.to_owned());
                cx.notify();
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
            .border_color(ThemeTokens::graphite().border())
            .child(panel)
    }

    fn render_restore_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_3()
            .w_full()
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(tokens.warning())
            .bg(tokens.surface_raised())
            .child(div().text_xs().text_color(tokens.text()).child(
                "Restoring this History entry replaces the current non-empty Text Diff session.",
            ))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        Button::new("Restore")
                            .variant(ButtonVariant::Primary)
                            .focus_handle(self.focus.history_confirm.clone())
                            .on_click(view_click(cx, |this, window, cx| {
                                this.confirm_restore(window, cx);
                            })),
                    )
                    .child(
                        Button::new("Cancel")
                            .focus_handle(self.focus.history_cancel.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                this.cancel_restore(cx);
                            })),
                    ),
            )
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
        let tokens = ThemeTokens::graphite();
        let mut main = div()
            .flex()
            .flex_col()
            .gap_3()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("Split")
                            .variant(if mode == DisplayMode::Split {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Secondary
                            })
                            .focus_handle(self.focus.split.clone())
                            .on_click(view_click(cx, |this, window, cx| {
                                this.set_mode(DisplayMode::Split, window, cx)
                            })),
                    )
                    .child(
                        Button::new("Unified")
                            .variant(if mode == DisplayMode::Unified {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Secondary
                            })
                            .focus_handle(self.focus.unified.clone())
                            .on_click(view_click(cx, |this, window, cx| {
                                this.set_mode(DisplayMode::Unified, window, cx)
                            })),
                    )
                    .child(
                        Button::new("Paste original")
                            .focus_handle(self.focus.paste_original.clone())
                            .on_click(view_click(cx, |this, window, cx| {
                                let old = &this.old;
                                this.paste(old, window, cx)
                            })),
                    )
                    .child(
                        Button::new("Paste updated")
                            .focus_handle(self.focus.paste_updated.clone())
                            .on_click(view_click(cx, |this, window, cx| {
                                let new = &this.new;
                                this.paste(new, window, cx)
                            })),
                    )
                    .child(div().flex_1())
                    .child(copy_feedback(self.copied.is_some(), "Copied to Clipboard"))
                    .child(
                        Button::new("Copy original")
                            .focus_handle(self.focus.copy_original.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                let text = this.old.text(cx);
                                this.copy(0, text, cx);
                            })),
                    )
                    .child(
                        Button::new("Copy updated")
                            .focus_handle(self.focus.copy_updated.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                let text = this.new.text(cx);
                                this.copy(1, text, cx);
                            })),
                    )
                    .child(
                        Button::new(if self.history_visible {
                            "History: on"
                        } else {
                            "History: off"
                        })
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
                    .gap_3()
                    .h_48()
                    .flex_shrink_0()
                    .child(div().flex().flex_1().min_h_0().child(panel(
                        "Original",
                        "neutral",
                        LabeledField::new("Original", old),
                    )))
                    .child(div().flex().flex_1().min_h_0().child(panel(
                        "Updated",
                        "neutral",
                        LabeledField::new("Updated", new),
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(WebDiffSurface::new(self.renderer.clone())),
            );
        if self.pending_restore.is_some() {
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
        if let Some(error) = self.history_error.clone() {
            main = main.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("History is paused for Text Diff: {error}"),
                None,
            ));
        }

        let mut root = div()
            .id("text-diff-workspace")
            .flex()
            .flex_row()
            .gap_3()
            .size_full()
            .child(main);
        if self.history_visible {
            root = root.child(
                div()
                    .flex()
                    .flex_col()
                    .min_h_0()
                    .border_l_1()
                    .border_color(tokens.border())
                    .child(self.render_history(cx)),
            );
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
