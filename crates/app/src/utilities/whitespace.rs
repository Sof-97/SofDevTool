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
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    DiagnosticSeverity, HistoryItem, HistoryPanel, TextEditor, ThemeTokens,
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
    actions: Vec<FocusHandle>,
    line_endings: Vec<FocusHandle>,
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
        let actions = (0..WhitespaceAction::ALL.len())
            .map(|_| cx.focus_handle().tab_stop(true).tab_index(0))
            .collect();
        let line_endings = (0..WhitespaceLineEnding::ALL.len())
            .map(|_| cx.focus_handle().tab_stop(true).tab_index(0))
            .collect();
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
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                actions,
                line_endings,
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
                self.result.set_text(output.clone(), window, cx);
            }
            _ => self.result.set_text("", window, cx),
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

    fn adjust_tab_width(&mut self, delta: i16, window: &mut Window, cx: &mut Context<Self>) {
        let next = (i16::from(self.tab_width) + delta)
            .clamp(i16::from(MIN_TAB_WIDTH), i16::from(MAX_TAB_WIDTH));
        self.tab_width = next as u8;
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
        snapshot: WhitespaceSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.action = snapshot.request.action;
        self.line_ending = snapshot.request.line_ending;
        self.tab_width = snapshot.request.tab_width;
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

    fn history_items(&self) -> Vec<HistoryItem> {
        self.history_view
            .entries
            .iter()
            .map(|entry| {
                let snapshot = decode_snapshot(entry);
                HistoryItem {
                    id: entry.id.clone(),
                    label: entry.captured_at.clone(),
                    preview: snapshot
                        .as_ref()
                        .map(|snapshot| preview_line(&snapshot.output))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn action_button(
        &self,
        index: usize,
        action: WhitespaceAction,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(action_label(action))
            .variant(if self.action == action {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(self.focus.actions[index].clone())
            .on_click(view_click(cx, move |this, window, cx| {
                this.set_action(action, window, cx);
            }))
    }

    fn line_ending_button(
        &self,
        index: usize,
        ending: WhitespaceLineEnding,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(ending.label())
            .variant(if self.line_ending == ending {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(self.focus.line_endings[index].clone())
            .on_click(view_click(cx, move |this, window, cx| {
                this.set_line_ending(ending, window, cx);
            }))
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
            "No retained operations yet.",
        )
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
            .child(panel)
    }

    fn render_restore_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::active();
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
                "Restoring this History entry replaces the current non-empty Whitespace session.",
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

        let mut actions_row = div().flex().flex_row().flex_wrap().gap_2();
        for (index, action) in WhitespaceAction::ALL.iter().copied().enumerate() {
            actions_row = actions_row.child(self.action_button(index, action, cx));
        }

        let mut options_row = div().flex().flex_row().items_center().gap_2();
        if self.action == WhitespaceAction::NormalizeLineEndings {
            options_row = options_row.child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Line ending"),
            );
            for (index, ending) in WhitespaceLineEnding::ALL.iter().copied().enumerate() {
                options_row = options_row.child(self.line_ending_button(index, ending, cx));
            }
        }
        if self.action.uses_tab_width() {
            options_row = options_row
                .child(
                    div()
                        .text_xs()
                        .text_color(tokens.text_muted())
                        .child(format!("Tab width: {}", self.tab_width)),
                )
                .child(
                    Button::new("Tab −")
                        .disabled(self.tab_width <= MIN_TAB_WIDTH)
                        .focus_handle(self.focus.tab_decrease.clone())
                        .on_click(view_click(cx, |this, window, cx| {
                            this.adjust_tab_width(-1, window, cx);
                        })),
                )
                .child(
                    Button::new("Tab +")
                        .disabled(self.tab_width >= MAX_TAB_WIDTH)
                        .focus_handle(self.focus.tab_increase.clone())
                        .on_click(view_click(cx, |this, window, cx| {
                            this.adjust_tab_width(1, window, cx);
                        })),
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
            )
            .child(
                Button::new("Paste")
                    .focus_handle(self.focus.paste.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::new("Copy Result")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::new("Clear")
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
            .p_4()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Whitespace Conversion"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Nine explicit actions, local and offline"),
                    ),
            )
            .child(actions_row)
            .child(options_row)
            .child(toolbar);

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
            .gap_4()
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
