//! The Whitespace Conversion workspace: nine explicit actions with line-ending
//! and tab-width options, an explicit Clipboard surface and fresh Rust History.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::whitespace::{
    Whitespace, WhitespaceAction, WhitespaceEvaluation, WhitespaceLineEnding, WhitespaceRequest,
    WhitespaceSnapshot, DEFAULT_TAB_WIDTH, MAX_TAB_WIDTH, MIN_TAB_WIDTH,
};
use sofdevtool_core::utility::Utility;
use sofui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ConfirmationBar,
    DiagnosticSeverity, NumericStepper, SegmentedControl, SegmentedControlFocus, SegmentedOption,
    SelectableList, SelectableListFocus, SelectableRow, TextEditor, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(200);

type WhitespaceSession = Session<Whitespace>;

/// Builds the Whitespace Conversion workspace as a type-erased view.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| WhitespaceWorkspace::new(window, cx, clipboard, history))
        .into()
}

struct ButtonFocus {
    tab_decrease: FocusHandle,
    tab_increase: FocusHandle,
    paste: FocusHandle,
    copy: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct WhitespaceWorkspace {
    input: TextEditor,
    result: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    action: WhitespaceAction,
    line_ending: WhitespaceLineEnding,
    tab_width: u8,
    session: WhitespaceSession,
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

impl WhitespaceWorkspace {
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
        let history_view = HistoryViewState::load(&history, Whitespace::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Whitespace::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        Self {
            input,
            result,
            clipboard,
            history,
            action: WhitespaceAction::EdgeTrim,
            line_ending: WhitespaceLineEnding::Lf,
            tab_width: DEFAULT_TAB_WIDTH,
            session: WhitespaceSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            history_visible: true,
            history_focus: SelectableListFocus::new(),
            choice_focus: SegmentedControlFocus::new(),
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                tab_decrease: cx.focus_handle().tab_stop(true).tab_index(0),
                tab_increase: cx.focus_handle().tab_stop(true).tab_index(0),
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

    fn request(&self, cx: &App) -> WhitespaceRequest {
        WhitespaceRequest {
            input: self.input.text(cx),
            action: self.action,
            line_ending: self.line_ending,
            tab_width: self.tab_width,
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
        let payload = serde_json::to_value(&snapshot).expect("a Whitespace snapshot serializes");
        let result = self
            .history
            .record(Whitespace::ID, Whitespace::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Whitespace::ID, result);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Whitespace::ID);
        cx.notify();
    }

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            WhitespaceEvaluation::Valid { output } => {
                self.result.assign_text(output.clone(), window, cx);
            }
            _ => self.result.assign_text("", window, cx),
        }
    }

    fn set_action(
        &mut self,
        action: WhitespaceAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.action = action;
        self.schedule(window, cx);
    }

    fn set_line_ending(
        &mut self,
        line_ending: WhitespaceLineEnding,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.line_ending = line_ending;
        self.schedule(window, cx);
    }

    fn adjust_tab_width(&mut self, delta: i32, window: &mut Window, cx: &mut Context<Self>) {
        let next = NumericStepper::stepped(
            i32::from(self.tab_width),
            delta,
            i32::from(MIN_TAB_WIDTH),
            i32::from(MAX_TAB_WIDTH),
        );
        self.tab_width = next as u8;
        self.schedule(window, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.input.edit_text(text, window, cx);
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
        self.input.edit_text("", window, cx);
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
        snapshot: WhitespaceSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.action = snapshot.request.action;
        self.line_ending = snapshot.request.line_ending;
        self.tab_width = snapshot.request.tab_width;
        self.input
            .assign_text(snapshot.request.input.clone(), window, cx);
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
                .retained(&self.history, Whitespace::ID, &entry)
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
                .retained(&self.history, Whitespace::ID, &entry)
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
        let actions = Button::with_id("whitespace.history.restore-selected", "Restore selected")
            .disabled(!restore_enabled)
            .focus_handle(self.focus.history_restore.clone())
            .on_click(view_click(cx, |this, window, cx| {
                this.restore_selected(window, cx);
            }));
        let weak = cx.weak_entity();
        let list = SelectableList::new(
            "whitespace.history",
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
            "whitespace.history.restore",
            "Restoring this History entry replaces the current non-empty Whitespace session.",
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

impl Render for WhitespaceWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), WhitespaceEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.is_empty())
                .unwrap_or(false);

        let actions_row = SegmentedControl::new(
            "whitespace.action",
            "Whitespace action",
            WhitespaceAction::ALL
                .into_iter()
                .map(|choice| SegmentedOption::new(format!("{choice:?}"), action_label(choice)))
                .collect(),
            Some(format!("{:?}", self.action)),
            self.choice_focus.clone(),
        )
        .on_change(Rc::new({
            let weak = cx.weak_entity();
            move |id, window, cx| {
                if let Some(choice) = WhitespaceAction::ALL
                    .into_iter()
                    .find(|choice| format!("{choice:?}") == id)
                {
                    weak.update(cx, |this, cx| this.set_action(choice, window, cx))
                        .ok();
                }
            }
        }));

        let mut options_row = div().flex().flex_row().items_center().gap_2();
        if self.action == WhitespaceAction::NormalizeLineEndings {
            let line_ending_control = SegmentedControl::new(
                "whitespace.line-ending",
                "Line ending",
                WhitespaceLineEnding::ALL
                    .into_iter()
                    .map(|choice| SegmentedOption::new(format!("{choice:?}"), choice.label()))
                    .collect(),
                Some(format!("{:?}", self.line_ending)),
                self.choice_focus.clone(),
            )
            .on_change(Rc::new({
                let weak = cx.weak_entity();
                move |id, window, cx| {
                    if let Some(choice) = WhitespaceLineEnding::ALL
                        .into_iter()
                        .find(|choice| format!("{choice:?}") == id)
                    {
                        weak.update(cx, |this, cx| this.set_line_ending(choice, window, cx))
                            .ok();
                    }
                }
            }));
            options_row = options_row.child(line_ending_control);
        }
        if self.action.uses_tab_width() {
            let weak = cx.weak_entity();
            options_row = options_row.child(
                NumericStepper::new(
                    "whitespace.tab-width",
                    "Tab width",
                    Some(i32::from(self.tab_width)),
                    i32::from(MIN_TAB_WIDTH),
                    i32::from(MAX_TAB_WIDTH),
                    1,
                )
                .focus_handles(
                    self.focus.tab_decrease.clone(),
                    self.focus.tab_increase.clone(),
                )
                .on_step(move |delta, window, cx| {
                    weak.update(cx, |this, cx| this.adjust_tab_width(delta, window, cx))
                        .ok();
                }),
            );
        }

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(div().flex_1())
            .child(copy_feedback(self.copied, "Copied to Clipboard"))
            .child(
                Button::with_id(
                    "whitespace.history.toggle",
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
                Button::with_id("whitespace.paste", "Paste")
                    .focus_handle(self.focus.paste.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::with_id("whitespace.copy-result", "Copy Result")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::with_id("whitespace.clear", "Clear")
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
            .child(actions_row)
            .child(options_row)
            .child(toolbar)
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Nine explicit actions; line ending and tab width apply where shown."),
            );

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("Whitespace Conversion History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            empty_state("Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), WhitespaceEvaluation::Empty) {
            empty_state("Paste or type text to begin").into_any_element()
        } else {
            self.result
                .render(true, "whitespace.result")
                .into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(panel(
                "Input",
                "exact text, endings preserved",
                self.input.render(false, "whitespace.input"),
            ))
            .child(panel("Result", "read-only, selectable", result_body));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics())
    }
}

fn action_label(action: WhitespaceAction) -> &'static str {
    match action {
        WhitespaceAction::EdgeTrim => "Edge Trim",
        WhitespaceAction::PerLineTrim => "Per-line Trim",
        WhitespaceAction::CollapseHorizontalWhitespace => "Collapse Horizontal Whitespace",
        WhitespaceAction::CollapseAllWhitespace => "Collapse All Whitespace",
        WhitespaceAction::NormalizeLineEndings => "Normalize Line Endings",
        WhitespaceAction::TabsToSpaces => "Tabs to Spaces",
        WhitespaceAction::SpacesToTabs => "Spaces to Tabs",
        WhitespaceAction::RemoveBlankLines => "Remove Blank Lines",
        WhitespaceAction::Dedent => "Dedent",
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<WhitespaceSnapshot> {
    if entry.snapshot_version != Whitespace::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

fn preview_line(output: &str) -> String {
    let single_line = output.replace(['\n', '\r'], " ");
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

    struct TestRoot(Entity<WhitespaceWorkspace>);

    impl Render for TestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
        }
    }

    #[gpui::test]
    fn tab_stop_stepper_line_endings_copy_and_restore(cx: &mut gpui::TestAppContext) {
        cx.update(sofui::init);
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-whitespace-redesign-{}-{}",
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
            let view = cx
                .new(|cx| WhitespaceWorkspace::new(window, cx, clipboard.clone(), history.clone()));
            captured = Some(view.clone());
            TestRoot(view)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input.edit_text("a\tb\n\t👩🏽\u{200D}💻", window, cx)
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let choices = workspace.read(cx).choice_focus.clone();
            window.focus(&choices.handle("whitespace.action", "TabsToSpaces", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            "a   b\n    👩🏽\u{200D}💻"
        );
        let saved = history.load(Whitespace::ID).unwrap();
        assert_eq!(saved[0].utility_id, Whitespace::ID);
        assert_eq!(workspace.read_with(&cx, |view, _| view.tab_width), 4);

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.focus(&workspace.read(cx).focus.tab_increase.clone(), cx);
        });
        cx.simulate_keystrokes("enter");
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(workspace.read_with(&cx, |view, _| view.tab_width), 5);
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            "a    b\n     👩🏽\u{200D}💻"
        );
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.focus(&workspace.read(cx).focus.copy.clone(), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            clipboard.0.borrow().as_deref(),
            Some("a    b\n     👩🏽\u{200D}💻")
        );

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input.edit_text("other", window, cx);
                assert!(view.history_view.select(&saved[0].id));
                view.restore_selected(window, cx);
                assert!(view.history_view.pending_restore.is_some());
                view.confirm_restore(window, cx);
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.action),
            WhitespaceAction::TabsToSpaces
        );
        assert_eq!(workspace.read_with(&cx, |view, _| view.tab_width), 4);
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            "a   b\n    👩🏽\u{200D}💻"
        );
        assert_eq!(
            history.load(Whitespace::ID).unwrap().len(),
            3,
            "restore must not record"
        );

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let choices = workspace.read(cx).choice_focus.clone();
            window.focus(
                &choices.handle("whitespace.action", "NormalizeLineEndings", cx),
                cx,
            );
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.input.edit_text("a\r\nb", window, cx));
            window.draw(cx).clear(cx);
            let choices = workspace.read(cx).choice_focus.clone();
            window.focus(&choices.handle("whitespace.line-ending", "Cr", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            "a\rb"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
