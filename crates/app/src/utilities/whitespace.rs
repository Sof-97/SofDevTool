//! The Whitespace Conversion workspace: nine explicit actions with line-ending
//! and tab-width options, an explicit Clipboard surface and fresh Rust History.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, Entity, IntoElement, Render, Subscription, Window};
use gpui_kit::component::{
    button::Button,
    input::{InputEvent, InputState, NumberInput, TextareaState},
    list::ListState,
    tab::{Tab, TabBar},
    ActiveTheme as _, Disableable as _,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::whitespace::{
    Whitespace, WhitespaceAction, WhitespaceEvaluation, WhitespaceLineEnding, WhitespaceRequest,
    WhitespaceSnapshot, DEFAULT_TAB_WIDTH, MAX_TAB_WIDTH, MIN_TAB_WIDTH,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::ui;
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

pub struct WhitespaceWorkspace {
    input: Entity<TextareaState>,
    result: Entity<TextareaState>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    action: WhitespaceAction,
    line_ending: WhitespaceLineEnding,
    tab_width: u8,
    tab_width_input: Entity<InputState>,
    session: WhitespaceSession,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl WhitespaceWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let input = cx.new(|cx| TextareaState::new(window, cx));
        let result = cx.new(|cx| TextareaState::new(window, cx));
        let mut subscriptions = vec![cx.subscribe_in(
            &input,
            window,
            |this, _entity, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.schedule(window, cx);
                }
            },
        )];
        let tab_width_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(DEFAULT_TAB_WIDTH.to_string())
                .min(f64::from(MIN_TAB_WIDTH))
                .max(f64::from(MAX_TAB_WIDTH))
                .step(1_f64)
        });
        subscriptions.push(cx.subscribe_in(
            &tab_width_input,
            window,
            |this, _entity, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.commit_tab_width(window, cx);
                }
            },
        ));
        let history_view = HistoryViewState::load(&history, Whitespace::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Whitespace::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        let weak = cx.weak_entity();
        let history_list = ui::history_state(
            window,
            cx,
            "No retained operations yet.",
            Rc::new(move |id, _window, cx| {
                weak.update(cx, |this, cx| {
                    if this.history_view.select(id) {
                        ui::history_set_selected(
                            &this.history_list,
                            this.history_view.selected.clone(),
                            cx,
                        );
                        cx.notify();
                    }
                })
                .ok();
            }),
        );
        let workspace = Self {
            input,
            result,
            clipboard,
            history,
            action: WhitespaceAction::EdgeTrim,
            line_ending: WhitespaceLineEnding::Lf,
            tab_width: DEFAULT_TAB_WIDTH,
            tab_width_input,
            session: WhitespaceSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            history_visible: true,
            history_list,
            _history_subscription: history_subscription,
            _subscriptions: subscriptions,
        };
        workspace.sync_history(cx);
        workspace
    }

    fn request(&self, cx: &App) -> WhitespaceRequest {
        WhitespaceRequest {
            input: self.input.read(cx).value().to_string(),
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
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Whitespace::ID);
        self.sync_history(cx);
        cx.notify();
    }

    /// Reflects the owning view's History rows and selection into the kit list.
    fn sync_history(&self, cx: &mut Context<Self>) {
        ui::history_set_rows(&self.history_list, self.history_items(), cx);
        ui::history_set_selected(&self.history_list, self.history_view.selected.clone(), cx);
    }

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        let value = match self.session.evaluation() {
            WhitespaceEvaluation::Valid { output } => output.clone(),
            _ => String::new(),
        };
        self.result
            .update(cx, |state, cx| state.set_value(value, window, cx));
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

    /// Reads the tab width from the number input, clamps it to the Utility
    /// bounds and reschedules with the clamped value.
    fn commit_tab_width(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.tab_width_input.read(cx).value().to_string();
        let parsed = raw.trim().parse::<u8>().unwrap_or(self.tab_width);
        let clamped = parsed.clamp(MIN_TAB_WIDTH, MAX_TAB_WIDTH);
        if clamped == self.tab_width {
            return;
        }
        self.tab_width = clamped;
        self.schedule(window, cx);
    }

    /// Sets the tab width, reflects it in the number input and reschedules.
    ///
    /// Used by restore, which must not let the input's own change listener
    /// record an incomplete edit; `apply_restore` runs it under
    /// `suppress_changes`.
    fn set_tab_width(&mut self, tab_width: u8, window: &mut Window, cx: &mut Context<Self>) {
        self.tab_width = tab_width;
        self.tab_width_input.update(cx, |state, cx| {
            state.set_value(tab_width.to_string(), window, cx)
        });
        self.schedule(window, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.input
                .update(cx, |state, cx| state.replace_all(text, window, cx));
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
        self.input
            .update(cx, |state, cx| state.replace_all("", window, cx));
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
        let current = self.input.read(cx).value().to_string();
        if !current.is_empty() && current != snapshot.request.input {
            self.history_view.pending_restore = Some(entry);
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current non-empty Whitespace session.",
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
        snapshot: WhitespaceSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.action = snapshot.request.action;
        self.line_ending = snapshot.request.line_ending;
        self.set_tab_width(snapshot.request.tab_width, window, cx);
        self.input.update(cx, |state, cx| {
            state.set_value(snapshot.request.input.clone(), window, cx)
        });
        self.session.restore(snapshot);
        self.display_epoch = u64::MAX;
        self.suppress_changes = false;
        self.history_view.pending_restore = None;
        self.copied = false;
        self.sync_history(cx);
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
                        .map(|snapshot| preview_line(&snapshot.output))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: snapshot.is_none().then(|| "Unavailable".to_owned()),
                    selectable: true,
                }
            })
            .collect()
    }

    fn render_diagnostics(&self, cx: &App) -> impl IntoElement {
        let mut column = div().flex().flex_col().gap_2().w_full();
        for diagnostic in self.session.evaluation().diagnostics() {
            let severity = match diagnostic.severity {
                sofdevtool_core::diagnostic::Severity::Error => ui::DiagnosticSeverity::Error,
                sofdevtool_core::diagnostic::Severity::Warning => ui::DiagnosticSeverity::Warning,
            };
            let location = diagnostic.location.map(|l| (l.line, l.column));
            column = column.child(ui::diagnostic_banner(
                cx,
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
        let restore = Button::new("whitespace.history.restore-selected")
            .label("Restore selected")
            .disabled(!restore_enabled)
            .on_click(cx.listener(|this, _event, window, cx| this.restore_selected(window, cx)));
        let theme = cx.theme();
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

impl Render for WhitespaceWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let theme = cx.theme().clone();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), WhitespaceEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.is_empty())
                .unwrap_or(false);

        let action_index = WhitespaceAction::ALL
            .iter()
            .position(|choice| *choice == self.action)
            .unwrap_or(0);
        let actions_row = TabBar::new("whitespace.action")
            .segmented()
            .selected_index(action_index)
            .children(
                WhitespaceAction::ALL
                    .into_iter()
                    .map(|choice| Tab::new().label(action_label(choice))),
            )
            .on_click(cx.listener(|this, index, window, cx| {
                if let Some(choice) = WhitespaceAction::ALL.get(*index) {
                    this.set_action(*choice, window, cx);
                }
            }));

        let mut options_row = div().flex().flex_row().items_center().gap_2();
        if self.action == WhitespaceAction::NormalizeLineEndings {
            let line_ending_index = WhitespaceLineEnding::ALL
                .iter()
                .position(|choice| *choice == self.line_ending)
                .unwrap_or(0);
            let line_ending_control = TabBar::new("whitespace.line-ending")
                .segmented()
                .selected_index(line_ending_index)
                .children(
                    WhitespaceLineEnding::ALL
                        .into_iter()
                        .map(|choice| Tab::new().label(choice.label())),
                )
                .on_click(cx.listener(|this, index, window, cx| {
                    if let Some(choice) = WhitespaceLineEnding::ALL.get(*index) {
                        this.set_line_ending(*choice, window, cx);
                    }
                }));
            options_row = options_row.child(line_ending_control);
        }
        if self.action.uses_tab_width() {
            options_row = options_row.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Tab width"),
                    )
                    .child(NumberInput::new(&self.tab_width_input)),
            );
        }

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(div().flex_1())
            .child(ui::copy_feedback(cx, self.copied, "Copied to Clipboard"))
            .child(
                Button::new("whitespace.history.toggle")
                    .label(if self.history_visible {
                        "History: on"
                    } else {
                        "History: off"
                    })
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.history_visible = !this.history_visible;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("whitespace.paste")
                    .label("Paste")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::new("whitespace.copy-result")
                    .label("Copy Result")
                    .disabled(!can_copy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::new("whitespace.clear")
                    .label("Clear")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.clear(window, cx);
                    })),
            );

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(theme.background)
            .text_color(theme.foreground)
            .gap_3()
            .child(actions_row)
            .child(options_row)
            .child(toolbar)
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Nine explicit actions; line ending and tab width apply where shown."),
            );

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Whitespace Conversion History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            ui::empty_state(cx, "Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), WhitespaceEvaluation::Empty) {
            ui::empty_state(cx, "Paste or type text to begin").into_any_element()
        } else {
            ui::multiline_editor(&self.result, true, "whitespace.result").into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(ui::panel(
                cx,
                "Input",
                "exact text, endings preserved",
                ui::multiline_editor(&self.input, false, "whitespace.input"),
            ))
            .child(ui::panel(
                cx,
                "Result",
                "read-only, selectable",
                result_body,
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics(cx))
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

    use gpui::VisualTestContext;
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

    #[gpui::test]
    fn tab_width_line_endings_copy_and_restore(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_kit::init);
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
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input.update(cx, |state, cx| {
                    state.replace_all("a\tb\n\t👩🏽\u{200D}💻", window, cx)
                })
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.set_action(WhitespaceAction::TabsToSpaces, window, cx)
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            "a   b\n    👩🏽\u{200D}💻"
        );
        let saved = history.load(Whitespace::ID).unwrap();
        assert_eq!(saved[0].utility_id, Whitespace::ID);
        assert_eq!(workspace.read_with(&cx, |view, _| view.tab_width), 4);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.set_tab_width(5, window, cx));
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(workspace.read_with(&cx, |view, _| view.tab_width), 5);
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            "a    b\n     👩🏽\u{200D}💻"
        );
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_result(cx));
        });
        assert_eq!(
            clipboard.0.borrow().as_deref(),
            Some("a    b\n     👩🏽\u{200D}💻")
        );

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input
                    .update(cx, |state, cx| state.replace_all("other", window, cx));
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
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            "a   b\n    👩🏽\u{200D}💻"
        );
        assert_eq!(
            history.load(Whitespace::ID).unwrap().len(),
            3,
            "restore must not record"
        );

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.set_action(WhitespaceAction::NormalizeLineEndings, window, cx)
            });
            window.draw(cx).clear(cx);
        });
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input
                    .update(cx, |state, cx| state.replace_all("a\r\nb", window, cx));
                view.set_line_ending(WhitespaceLineEnding::Cr, window, cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            "a\rb"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
