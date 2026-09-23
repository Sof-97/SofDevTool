//! The JSON Utility workspace: input, format/minify/query controls, diagnostics,
//! an explicit Clipboard surface and fresh Rust History.
//!
//! Input changes are debounced and revision-gated through the shared session, so
//! an obsolete asynchronous completion can never publish a result. One settled
//! valid operation is recorded once; preview and restore never reevaluate.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::json::{
    Indentation, Json, JsonEvaluation, JsonMode, JsonRequest, JsonSession, JsonSnapshot, Severity,
};
use sofdevtool_core::session::SubmitOutcome;
use sofdevtool_core::utility::Utility;
use sofui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    ConfirmationBar, DiagnosticSeverity, LabeledField, SegmentedControl, SegmentedControlFocus,
    SegmentedOption, SelectableList, SelectableListFocus, SelectableRow, TextEditor, TextField,
    ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};

const DEBOUNCE: Duration = Duration::from_millis(250);

struct ButtonFocus {
    indent: FocusHandle,
    sort: FocusHandle,
    paste: FocusHandle,
    copy: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct JsonWorkspace {
    input: TextEditor,
    result: TextEditor,
    query: TextField,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    mode: JsonMode,
    indentation: Indentation,
    sort_keys: bool,
    session: JsonSession,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    history_focus: SelectableListFocus,
    choice_focus: SegmentedControlFocus,
    _history_subscription: HistorySubscription,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl JsonWorkspace {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let input = TextEditor::new(window, cx);
        let result = TextEditor::new(window, cx);
        let query = TextField::new(window, cx);

        let subscriptions = vec![
            input.on_change_in(window, cx, |this, window, cx| {
                this.schedule(window, cx);
            }),
            query.on_change_in(window, cx, |this, window, cx| {
                this.schedule(window, cx);
            }),
        ];

        let history_view = HistoryViewState::load(&history, Json::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Json::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });

        Self {
            input,
            result,
            query,
            clipboard,
            history,
            mode: JsonMode::Format,
            indentation: Indentation::TwoSpaces,
            sort_keys: false,
            session: JsonSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            history_visible: true,
            history_focus: SelectableListFocus::new(),
            choice_focus: SegmentedControlFocus::new(),
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                indent: cx.focus_handle().tab_stop(true).tab_index(0),
                sort: cx.focus_handle().tab_stop(true).tab_index(0),
                paste: cx.focus_handle().tab_stop(true).tab_index(0),
                copy: cx.focus_handle().tab_stop(true).tab_index(0),
                clear: cx.focus_handle().tab_stop(true).tab_index(0),
                history_toggle: cx.focus_handle().tab_stop(true).tab_index(0),
                history_restore: cx.focus_handle().tab_stop(true).tab_index(0),
                history_confirm: cx.focus_handle().tab_stop(true).tab_index(0),
                history_cancel: cx.focus_handle().tab_stop(true).tab_index(0),
            },
            _subscriptions: subscriptions,
        }
    }

    fn request(&self, cx: &App) -> JsonRequest {
        JsonRequest {
            input: self.input.text(cx),
            mode: self.mode,
            indentation: self.indentation,
            sort_keys: self.sort_keys,
            query: self.query.text(cx),
        }
    }

    /// Submits the current request. A changed request clears the visible result
    /// immediately and schedules a debounced, revision-gated evaluation.
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.suppress_changes {
            return;
        }
        let request = self.request(cx);
        let SubmitOutcome::Scheduled(revision) = self.session.submit(request) else {
            return;
        };
        self.sync_display(window, cx);
        cx.notify();

        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            executor.timer(DEBOUNCE).await;
            this.update(cx, |this, cx| {
                if this.session.resolve(revision).is_some() {
                    this.record_settled(cx);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// Records the one settled valid operation for the current revision, if any.
    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a JSON snapshot serializes");
        let result = self
            .history
            .record(Json::ID, Json::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Json::ID, result);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Json::ID);
        cx.notify();
    }

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            JsonEvaluation::Valid { output } => {
                self.result.assign_text(output.clone(), window, cx);
            }
            _ => self.result.assign_text("", window, cx),
        }
    }

    fn set_mode(&mut self, mode: JsonMode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        self.schedule(window, cx);
    }

    fn toggle_indentation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.indentation = match self.indentation {
            Indentation::TwoSpaces => Indentation::FourSpaces,
            Indentation::FourSpaces => Indentation::TwoSpaces,
        };
        self.schedule(window, cx);
    }

    fn toggle_sort(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sort_keys = !self.sort_keys;
        self.schedule(window, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.input.edit_text(text, window, cx);
        }
    }

    fn copy_result(&mut self, cx: &mut Context<Self>) {
        if let JsonEvaluation::Valid { output } = self.session.evaluation() {
            let output = output.clone();
            self.clipboard.write_text(&output, cx);
            self.copied = true;
            cx.notify();
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = false;
        self.input.edit_text("", window, cx);
    }

    /// Requests a restore. Replacing a different nonempty session is confirmed
    /// first; otherwise the snapshot applies immediately.
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
        let current = self.input.text(cx);
        if !current.trim().is_empty() && current != snapshot.request.input {
            self.history_view.pending_restore = Some(entry);
            cx.notify();
        } else {
            self.apply_restore(snapshot, window, cx);
        }
    }

    /// Applies a captured snapshot directly. It never reevaluates input, never
    /// records, and invalidates any pending completion from the prior revision.
    fn apply_restore(
        &mut self,
        snapshot: JsonSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.mode = snapshot.request.mode;
        self.indentation = snapshot.request.indentation;
        self.sort_keys = snapshot.request.sort_keys;
        self.input
            .assign_text(snapshot.request.input.clone(), window, cx);
        self.query
            .assign_text(snapshot.request.query.clone(), window, cx);
        self.session.restore(snapshot);
        self.display_epoch = u64::MAX;
        self.suppress_changes = false;
        self.history_view.pending_restore = None;
        self.copied = false;
        self.sync_display(window, cx);
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.history_view.pending_restore.clone() {
            let retained = self.history_view.retained(&self.history, Json::ID, &entry);
            if retained {
                let Some(snapshot) = decode_snapshot(&entry) else {
                    return;
                };
                self.apply_restore(snapshot, window, cx);
            } else {
                self.reconcile_history(cx);
            }
        }
    }

    fn cancel_restore(&mut self, cx: &mut Context<Self>) {
        self.history_view.pending_restore = None;
        cx.notify();
    }

    fn select_history(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.history_view.select(id) {
            cx.notify();
        }
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
            if self.history_view.retained(&self.history, Json::ID, &entry) {
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
                        .map(|snapshot| preview_line(&snapshot.output))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: snapshot
                        .is_none()
                        .then(|| "Unavailable snapshot".to_owned()),
                    selectable: true,
                }
            })
            .collect()
    }

    fn render_diagnostics(&self) -> impl IntoElement {
        let mut column = div().flex().flex_col().gap_2().w_full();
        for diagnostic in self.session.evaluation().diagnostics() {
            let severity = match diagnostic.severity {
                Severity::Error => DiagnosticSeverity::Error,
                Severity::Warning => DiagnosticSeverity::Warning,
            };
            let location = diagnostic.location.map(|l| (l.line, l.column));
            column = column.child(diagnostic_banner(severity, &diagnostic.message, location));
        }
        column
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
            Button::with_id("json.history.restore-selected", "Restore selected")
                .disabled(!restore_enabled)
                .focus_handle(self.focus.history_restore.clone())
                .on_click(view_click(cx, |this, window, cx| {
                    this.restore_selected(window, cx);
                })),
        );

        let items = self.history_items();
        let count = items.len();
        let panel = SelectableList::new(
            "json.history",
            "History",
            items,
            selected,
            "No retained operations yet.",
            self.history_focus.clone(),
        )
        .summary(format!("{count}/25"))
        .on_select(Rc::new({
            let weak = cx.weak_entity();
            move |id, _window, cx| {
                weak.update(cx, |this, cx| this.select_history(id, cx)).ok();
            }
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
            "json.history.restore",
            "Restoring this History entry replaces the current non-empty JSON session.",
            "Restore",
            "Cancel",
        )
        .focus_handles(
            self.focus.history_confirm.clone(),
            self.focus.history_cancel.clone(),
        )
        .on_confirm(view_click(cx, |this, window, cx| {
            this.confirm_restore(window, cx);
        }))
        .on_cancel(view_click(cx, |this, _window, cx| {
            this.cancel_restore(cx);
        }))
    }
}

impl Render for JsonWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);

        let tokens = ThemeTokens::active();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), JsonEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.trim().is_empty())
                .unwrap_or(false);

        let selected_mode = match self.mode {
            JsonMode::Format => "format",
            JsonMode::Minify => "minify",
            JsonMode::Query => "query",
        };
        let mode_control = SegmentedControl::new(
            "json.mode",
            "JSON operation",
            vec![
                SegmentedOption::new("format", "Format"),
                SegmentedOption::new("minify", "Minify"),
                SegmentedOption::new("query", "Query"),
            ],
            Some(selected_mode.to_owned()),
            self.choice_focus.clone(),
        )
        .on_change(Rc::new({
            let weak = cx.weak_entity();
            move |id, window, cx| {
                let mode = match id {
                    "format" => JsonMode::Format,
                    "minify" => JsonMode::Minify,
                    "query" => JsonMode::Query,
                    _ => return,
                };
                weak.update(cx, |this, cx| this.set_mode(mode, window, cx))
                    .ok();
            }
        }));
        let mut toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(mode_control);

        if self.mode == JsonMode::Format {
            toolbar = toolbar.child(
                Button::with_id(
                    "json.indentation",
                    match self.indentation {
                        Indentation::TwoSpaces => "Indent: 2",
                        Indentation::FourSpaces => "Indent: 4",
                    },
                )
                .focus_handle(self.focus.indent.clone())
                .on_click(view_click(cx, |this, window, cx| {
                    this.toggle_indentation(window, cx);
                })),
            );
        }

        toolbar = toolbar
            .child(
                Button::with_id(
                    "json.sort-keys",
                    if self.sort_keys {
                        "Sort keys: on"
                    } else {
                        "Sort keys: off"
                    },
                )
                .variant(if self.sort_keys {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                })
                .focus_handle(self.focus.sort.clone())
                .on_click(view_click(cx, |this, window, cx| {
                    this.toggle_sort(window, cx);
                })),
            )
            .child(div().flex_1())
            .child(copy_feedback(self.copied, "Copied to Clipboard"))
            .child(
                Button::with_id(
                    "json.history.toggle",
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
            )
            .child(
                Button::with_id("json.paste", "Paste")
                    .focus_handle(self.focus.paste.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::with_id("json.copy-result", "Copy Result")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::with_id("json.clear", "Clear")
                    .focus_handle(self.focus.clear.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.clear(window, cx);
                    })),
            );

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(tokens.background())
            .text_color(tokens.text())
            .gap_3()
            .child(toolbar);

        if self.mode == JsonMode::Query {
            column = column.child(
                LabeledField::new("Path", self.query.render("json.query"))
                    .hint("JSON Pointer (/a/b) or dot/bracket (a.b[0])"),
            );
        }

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("JSON History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            empty_state("Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), JsonEvaluation::Empty) {
            empty_state("Paste JSON to begin").into_any_element()
        } else {
            self.result.render(true, "json.result").into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(panel(
                "Input",
                "live validation",
                self.input.render(false, "json.input"),
            ))
            .child(panel("Result", "read-only, selectable", result_body));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }

        column.child(workspace).child(self.render_diagnostics())
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<JsonSnapshot> {
    if entry.snapshot_version != Json::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

/// A short, single-line preview of a retained output.
fn preview_line(output: &str) -> String {
    let single_line = output.replace('\n', " ");
    let trimmed = single_line.trim();
    let mut preview: String = trimmed.chars().take(80).collect();
    if trimmed.chars().count() > 80 {
        preview.push('…');
    }
    if preview.is_empty() {
        "Empty result".to_owned()
    } else {
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use gpui::{Entity, VisualTestContext};

    use crate::history::{HistoryStore, SystemClock};

    struct TestClipboard;

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            None
        }

        fn write_text(&self, _text: &str, _cx: &mut App) {}
    }

    struct TestRoot(Entity<JsonWorkspace>);

    impl Render for TestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
        }
    }

    fn isolated_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-json-list-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[gpui::test]
    fn selectable_history_and_confirmation_reconcile_after_settings_deletion(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(sofui::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| {
                JsonWorkspace::new(window, cx, Rc::new(TestClipboard), Rc::clone(&history))
            });
            captured = Some(view.clone());
            TestRoot(view)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).choice_focus.clone();
            window.focus(&focus.handle("json.mode", "minify", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            JsonMode::Minify
        );
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).choice_focus.clone();
            window.focus(&focus.handle("json.mode", "format", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            JsonMode::Format
        );
        for text in [r#"{"first":1}"#, r#"{"second":2}"#] {
            cx.update(|window, cx| {
                workspace.update(cx, |view, cx| view.input.edit_text(text, window, cx));
                window.draw(cx).clear(cx);
            });
            cx.executor().advance_clock(DEBOUNCE);
            cx.run_until_parked();
        }
        let entries = history.load(Json::ID).unwrap();
        assert_eq!(entries.len(), 2);
        let first_id = entries[1].id.clone();
        let second_id = entries[0].id.clone();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let list = workspace.read(cx).history_focus.clone();
            window.focus(&list.handle(&first_id, cx), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.history_view.selected.clone()),
            Some(first_id.clone())
        );
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.history_restore.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert!(workspace.read_with(&cx, |view, _| view.history_view.pending_restore.is_some()));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.history_cancel.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert!(workspace.read_with(&cx, |view, _| view.history_view.pending_restore.is_none()));
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.input.text(cx)),
            r#"{"second":2}"#
        );

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.history_restore.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.history_confirm.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.input.text(cx)),
            r#"{"first":1}"#
        );
        assert_eq!(history.load(Json::ID).unwrap().len(), 2);

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let list = workspace.read(cx).history_focus.clone();
            window.focus(&list.handle(&second_id, cx), cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.history_restore.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert!(workspace.read_with(&cx, |view, _| view.history_view.pending_restore.is_some()));
        cx.update(|_window, cx| history.clear_utility(Json::ID, cx).unwrap());
        assert!(workspace.read_with(&cx, |view, _| view.history_view.entries.is_empty()));
        assert!(workspace.read_with(&cx, |view, _| view.history_view.selected.is_none()));
        assert!(workspace.read_with(&cx, |view, _| view.history_view.pending_restore.is_none()));
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.input.text(cx)),
            r#"{"first":1}"#
        );
        fs::remove_dir_all(root).unwrap();
    }
}
