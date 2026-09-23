//! The Case Conversion workspace: inspect detected words and convert developer
//! text through the nine styles with an explicit Clipboard surface and fresh
//! Rust History.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::case_conversion::{
    CaseConversion, CaseConversionEvaluation, CaseConversionRequest, CaseConversionSnapshot,
    CaseConversionStyle,
};
use sofdevtool_core::utility::Utility;
use sofdevtool_ui::{
    copy_feedback, empty_state, panel, view_click, Button, ConfirmationBar, DiagnosticSeverity,
    SegmentedControl, SegmentedControlFocus, SegmentedOption, SelectableList, SelectableListFocus,
    SelectableRow, TextEditor, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(200);

type CaseConversionSession = Session<CaseConversion>;

/// Builds the Case Conversion workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| CaseConversionWorkspace::new(window, cx, clipboard, history))
        .into()
}

/// One distinct focus handle per simultaneously-rendered button. Reusing a
/// handle across two visible buttons aborts GPUI when both request focus in a
/// single frame.
struct ButtonFocus {
    paste: FocusHandle,
    copy: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct CaseConversionWorkspace {
    input: TextEditor,
    result: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    style: CaseConversionStyle,
    session: CaseConversionSession,
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

impl CaseConversionWorkspace {
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
        let history_view = HistoryViewState::load(&history, CaseConversion::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(CaseConversion::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        Self {
            input,
            result,
            clipboard,
            history,
            style: CaseConversionStyle::Camel,
            session: CaseConversionSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            history_visible: true,
            history_focus: SelectableListFocus::new(),
            choice_focus: SegmentedControlFocus::new(),
            _history_subscription: history_subscription,
            focus: ButtonFocus {
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

    fn request(&self, cx: &App) -> CaseConversionRequest {
        CaseConversionRequest {
            input: self.input.text(cx),
            style: self.style,
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
        let payload =
            serde_json::to_value(&snapshot).expect("a Case Conversion snapshot serializes");
        let result = self.history.record(
            CaseConversion::ID,
            CaseConversion::SNAPSHOT_VERSION,
            payload,
        );
        self.history_view
            .apply_record(&self.history, CaseConversion::ID, result);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view
            .reconcile(&self.history, CaseConversion::ID);
        cx.notify();
    }

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            CaseConversionEvaluation::Valid { output, .. } => {
                self.result.set_text(output.clone(), window, cx);
            }
            _ => self.result.set_text("", window, cx),
        }
    }

    fn set_style(
        &mut self,
        style: CaseConversionStyle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.style = style;
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
        snapshot: CaseConversionSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.style = snapshot.request.style;
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
                .retained(&self.history, CaseConversion::ID, &entry)
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
                .retained(&self.history, CaseConversion::ID, &entry)
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
                    status: snapshot.is_none().then(|| "Unavailable".to_owned()),
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
            column = column.child(sofdevtool_ui::diagnostic_banner(
                severity,
                &diagnostic.message,
                location,
            ));
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
        let actions = Button::with_id(
            "case-conversion.history.restore-selected",
            "Restore selected",
        )
        .disabled(!restore_enabled)
        .focus_handle(self.focus.history_restore.clone())
        .on_click(view_click(cx, |this, window, cx| {
            this.restore_selected(window, cx);
        }));
        let weak = cx.weak_entity();
        let list = SelectableList::new(
            "case-conversion.history",
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
            "case-conversion.history.restore",
            "Restoring this History entry replaces the current non-empty Case Conversion session.",
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

impl Render for CaseConversionWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), CaseConversionEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.is_empty())
                .unwrap_or(false);

        let styles = SegmentedControl::new(
            "case-conversion.style",
            "Case style",
            CaseConversionStyle::ALL
                .into_iter()
                .map(|choice| SegmentedOption::new(format!("{choice:?}"), choice.label()))
                .collect(),
            Some(format!("{:?}", self.style)),
            self.choice_focus.clone(),
        )
        .on_change(Rc::new({
            let weak = cx.weak_entity();
            move |id, window, cx| {
                if let Some(choice) = CaseConversionStyle::ALL
                    .into_iter()
                    .find(|choice| format!("{choice:?}") == id)
                {
                    weak.update(cx, |this, cx| this.set_style(choice, window, cx))
                        .ok();
                }
            }
        }));

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(div().flex_1())
            .child(copy_feedback(self.copied, "Copied to Clipboard"))
            .child(
                Button::with_id(
                    "case-conversion.history.toggle",
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
                Button::with_id("case-conversion.paste", "Paste")
                    .focus_handle(self.focus.paste.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::with_id("case-conversion.copy-result", "Copy Result")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::with_id("case-conversion.clear", "Clear")
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
            .child(styles)
            .child(toolbar)
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Nine styles share one grapheme-safe word segmentation."),
            );

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(sofdevtool_ui::diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("Case Conversion History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            empty_state("Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), CaseConversionEvaluation::Empty) {
            empty_state("Paste or type text to begin").into_any_element()
        } else {
            self.result
                .render(true, "case-conversion.result")
                .into_any_element()
        };

        let words = self.session.evaluation().words();
        let words_text = if words.is_empty() {
            "—".to_owned()
        } else {
            words.join(" · ")
        };
        let detected_words = div()
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(tokens.border())
            .bg(tokens.surface())
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Detected Words"),
            )
            .child(div().text_sm().text_color(tokens.text()).child(words_text));

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(panel(
                "Input",
                "any text",
                self.input.render(false, "case-conversion.input"),
            ))
            .child(panel("Result", "read-only, selectable", result_body));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column
            .child(workspace)
            .child(detected_words)
            .child(self.render_diagnostics())
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<CaseConversionSnapshot> {
    if entry.snapshot_version != CaseConversion::SNAPSHOT_VERSION {
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

    struct TestRoot(Entity<CaseConversionWorkspace>);

    impl Render for TestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
        }
    }

    #[gpui::test]
    fn unicode_style_keyboard_copy_and_exact_history_restore(cx: &mut gpui::TestAppContext) {
        cx.update(sofdevtool_ui::init);
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-case-redesign-{}-{}",
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
                CaseConversionWorkspace::new(window, cx, clipboard.clone(), history.clone())
            });
            captured = Some(view.clone());
            TestRoot(view)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input.edit_text("Cafe\u{301}Bar", window, cx)
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            "cafe\u{301}Bar"
        );
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.session.evaluation().words().to_vec()),
            ["Cafe\u{301}".to_owned(), "Bar".to_owned()]
        );
        let original = history.load(CaseConversion::ID).unwrap();
        assert_eq!(original.len(), 1);

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let choices = workspace.read(cx).choice_focus.clone();
            window.focus(&choices.handle("case-conversion.style", "Snake", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            "cafe\u{301}_bar"
        );
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.focus(&workspace.read(cx).focus.copy.clone(), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().as_deref(), Some("cafe\u{301}_bar"));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input.edit_text("other", window, cx);
                assert!(view.history_view.select(&original[0].id));
                view.restore_selected(window, cx);
                assert!(view.history_view.pending_restore.is_some());
                view.confirm_restore(window, cx);
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.style),
            CaseConversionStyle::Camel
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            "cafe\u{301}Bar"
        );
        assert_eq!(history.load(CaseConversion::ID).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
