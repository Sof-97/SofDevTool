//! The URL Encoding workspace: convert UTF-8 text through separate Path Segment
//! and Query Value modes with an explicit Clipboard surface and fresh Rust
//! History.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::url_encoding::{
    UrlEncoding, UrlEncodingDirection, UrlEncodingEvaluation, UrlEncodingMode, UrlEncodingRequest,
    UrlEncodingSnapshot,
};
use sofdevtool_core::utility::Utility;
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ConfirmationBar,
    DiagnosticSeverity, SegmentedControl, SegmentedControlFocus, SegmentedOption, SelectableList,
    SelectableListFocus, SelectableRow, TextEditor, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(200);

type UrlEncodingSession = Session<UrlEncoding>;

/// Builds the URL Encoding workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| UrlEncodingWorkspace::new(window, cx, clipboard, history))
        .into()
}

struct ButtonFocus {
    swap: FocusHandle,
    paste: FocusHandle,
    copy: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct UrlEncodingWorkspace {
    input: TextEditor,
    result: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    direction: UrlEncodingDirection,
    mode: UrlEncodingMode,
    session: UrlEncodingSession,
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

impl UrlEncodingWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let input = TextEditor::new(window, cx);
        let result = TextEditor::new(window, cx);
        let subscriptions = vec![input.on_change_in(window, cx, |this, window, cx| {
            this.schedule(window, cx);
        })];
        let history_view = HistoryViewState::load(&history, UrlEncoding::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(UrlEncoding::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        Self {
            input,
            result,
            clipboard,
            history,
            direction: UrlEncodingDirection::Encode,
            mode: UrlEncodingMode::PathSegment,
            session: UrlEncodingSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            history_visible: true,
            history_focus: SelectableListFocus::new(),
            choice_focus: SegmentedControlFocus::new(),
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                swap: cx.focus_handle().tab_stop(true).tab_index(0),
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

    fn request(&self, cx: &App) -> UrlEncodingRequest {
        UrlEncodingRequest {
            input: self.input.text(cx),
            direction: self.direction,
            mode: self.mode,
        }
    }

    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.suppress_changes {
            return;
        }
        let SubmitOutcome::Scheduled(revision) = self.session.submit(self.request(cx)) else {
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

    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a URL Encoding snapshot serializes");
        let result = self
            .history
            .record(UrlEncoding::ID, UrlEncoding::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, UrlEncoding::ID, result);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, UrlEncoding::ID);
        cx.notify();
    }

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            UrlEncodingEvaluation::Valid { output } => {
                self.result.set_text(output.clone(), window, cx);
            }
            _ => self.result.set_text("", window, cx),
        }
    }

    fn set_direction(
        &mut self,
        direction: UrlEncodingDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.direction = direction;
        self.schedule(window, cx);
    }

    fn set_mode(&mut self, mode: UrlEncodingMode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        self.schedule(window, cx);
    }

    fn swap(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(prior_output) = self.session.evaluation().output().map(str::to_owned) else {
            return;
        };
        if prior_output.is_empty() {
            return;
        }
        self.direction = match self.direction {
            UrlEncodingDirection::Encode => UrlEncodingDirection::Decode,
            UrlEncodingDirection::Decode => UrlEncodingDirection::Encode,
        };
        self.suppress_changes = true;
        self.input.replace_all(prior_output, window, cx);
        self.suppress_changes = false;
        self.copied = false;
        self.schedule(window, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.input.replace_all(text, window, cx);
        }
    }

    fn copy_result(&mut self, cx: &mut Context<Self>) {
        if let Some(output) = self.session.evaluation().output() {
            let output = output.to_owned();
            self.clipboard.write_text(&output, cx);
            self.copied = true;
            cx.notify();
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = false;
        self.input.replace_all("", window, cx);
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
        let current = self.input.text(cx);
        if !current.is_empty() && current != snapshot.request.input {
            self.history_view.pending_restore = Some(entry);
            cx.notify();
        } else {
            self.apply_restore(snapshot, window, cx);
        }
    }

    fn apply_restore(
        &mut self,
        snapshot: UrlEncodingSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.direction = snapshot.request.direction;
        self.mode = snapshot.request.mode;
        self.input
            .set_text(snapshot.request.input.clone(), window, cx);
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
            if self
                .history_view
                .retained(&self.history, UrlEncoding::ID, &entry)
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
                .retained(&self.history, UrlEncoding::ID, &entry)
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
                        .map(|snapshot| preview_line(&snapshot.output))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: (!snapshot.is_some()).then(|| "Unavailable".to_owned()),
                    selectable: true,
                }
            })
            .collect()
    }

    fn render_diagnostics(&self) -> impl IntoElement {
        let mut column = div().flex().flex_col().gap_2().w_full();
        for diagnostic in self.session.evaluation().diagnostics() {
            let severity = match diagnostic.severity {
                sofdevtool_core::diagnostic::Severity::Error => DiagnosticSeverity::Error,
                sofdevtool_core::diagnostic::Severity::Warning => DiagnosticSeverity::Warning,
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
            .is_some_and(|entry| decode_snapshot(entry).is_some());
        let actions = Button::with_id("url-encoding.history.restore-selected", "Restore selected")
            .disabled(!restore_enabled)
            .focus_handle(self.focus.history_restore.clone())
            .on_click(view_click(cx, |this, window, cx| {
                this.restore_selected(window, cx)
            }));
        let weak = cx.weak_entity();
        let list = SelectableList::new(
            "url-encoding.history",
            "History",
            self.history_items(),
            selected,
            "No retained operations yet.",
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
            .child(list)
    }

    fn render_restore_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        ConfirmationBar::new(
            "url-encoding.history.restore",
            "Restoring this History entry replaces the current non-empty URL Encoding session.",
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

impl Render for UrlEncodingWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), UrlEncodingEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.is_empty())
                .unwrap_or(false);

        let selected_direction = match self.direction {
            UrlEncodingDirection::Encode => "encode",
            UrlEncodingDirection::Decode => "decode",
        };
        let direction_control = SegmentedControl::new(
            "url-encoding.direction",
            "URL operation",
            vec![
                SegmentedOption::new("encode", "Encode"),
                SegmentedOption::new("decode", "Decode"),
            ],
            Some(selected_direction.to_owned()),
            self.choice_focus.clone(),
        )
        .on_change(Rc::new({
            let weak = cx.weak_entity();
            move |id, window, cx| {
                let direction = match id {
                    "encode" => UrlEncodingDirection::Encode,
                    "decode" => UrlEncodingDirection::Decode,
                    _ => return,
                };
                weak.update(cx, |this, cx| this.set_direction(direction, window, cx))
                    .ok();
            }
        }));
        let selected_mode = match self.mode {
            UrlEncodingMode::PathSegment => "path",
            UrlEncodingMode::QueryValue => "query",
        };
        let mode_control = SegmentedControl::new(
            "url-encoding.mode",
            "URL component",
            vec![
                SegmentedOption::new("path", "Path Segment"),
                SegmentedOption::new("query", "Query Value"),
            ],
            Some(selected_mode.to_owned()),
            self.choice_focus.clone(),
        )
        .on_change(Rc::new({
            let weak = cx.weak_entity();
            move |id, window, cx| {
                let mode = match id {
                    "path" => UrlEncodingMode::PathSegment,
                    "query" => UrlEncodingMode::QueryValue,
                    _ => return,
                };
                weak.update(cx, |this, cx| this.set_mode(mode, window, cx))
                    .ok();
            }
        }));
        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(direction_control)
            .child(mode_control)
            .child(
                Button::with_id("url-encoding.swap", "Swap")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.swap.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.swap(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(copy_feedback(self.copied, "Copied to Clipboard"))
            .child(
                Button::with_id(
                    "url-encoding.history.toggle",
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
                Button::with_id("url-encoding.paste", "Paste")
                    .focus_handle(self.focus.paste.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::with_id("url-encoding.copy-result", "Copy Result")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::with_id("url-encoding.clear", "Clear")
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
            .child(toolbar)
            .child(div().text_xs().text_color(tokens.text_muted()).child(
                "Spaces use %20. Decode keeps + as a literal plus; this is not form \
                         encoding or URL-safe Base64.",
            ));

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("URL Encoding History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            empty_state("Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), UrlEncodingEvaluation::Empty) {
            empty_state("Paste or type text to begin").into_any_element()
        } else {
            self.result
                .render(true, "url-encoding.result")
                .into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(panel(
                "UTF-8 Input",
                "exact bytes",
                self.input.render(false, "url-encoding.input"),
            ))
            .child(panel("Result", "read-only, selectable", result_body));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics())
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<UrlEncodingSnapshot> {
    if entry.snapshot_version != UrlEncoding::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

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
mod interaction_tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use gpui::{Entity, VisualTestContext};

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

    struct TestRoot(Entity<UrlEncodingWorkspace>);

    impl Render for TestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
        }
    }

    #[gpui::test]
    fn keyboard_mode_changes_percent_bytes_and_invalid_copy_stays_blocked(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(sofdevtool_ui::init);
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-url-redesign-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let clipboard = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| {
                UrlEncodingWorkspace::new(window, cx, clipboard.clone(), history.clone())
            });
            captured = Some(view.clone());
            TestRoot(view)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.input.edit_text("café +", window, cx));
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            "caf%C3%A9%20+"
        );
        assert_eq!(history.load(UrlEncoding::ID).unwrap().len(), 1);

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).choice_focus.clone();
            window.focus(&focus.handle("url-encoding.mode", "query", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            "caf%C3%A9%20%2B"
        );
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.copy.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().as_deref(), Some("caf%C3%A9%20%2B"));

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).choice_focus.clone();
            window.focus(&focus.handle("url-encoding.direction", "decode", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.input.edit_text("%ZZ", window, cx));
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert!(!workspace.read_with(&cx, |view, _| view
            .session
            .evaluation()
            .is_valid_operation()));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.copy.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().as_deref(), Some("caf%C3%A9%20%2B"));
        fs::remove_dir_all(root).unwrap();
    }
}
