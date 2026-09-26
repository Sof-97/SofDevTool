//! The Timestamps workspace: explicit Auto/manual inference, a named-zone
//! control, an injected-Now action, synchronized read-only representations and
//! fresh Rust History. Restoring never reads the clock again.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::timestamps::{
    SystemTimestampSource, TimestampMode, TimestampOrigin, TimestampRepresentations, Timestamps,
    TimestampsEvaluation, TimestampsRequest, TimestampsSnapshot,
};
use sofdevtool_core::utility::Utility;
use sofui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ConfirmationBar,
    DiagnosticSeverity, SegmentedControl, SegmentedControlFocus, SegmentedOption, SelectableList,
    SelectableListFocus, SelectableRow, TextEditor, TextField, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(200);
const DEFAULT_ZONE: &str = "UTC";

type TimestampsSession = Session<Timestamps>;

/// Builds the Timestamps workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| TimestampsWorkspace::new(window, cx, clipboard, history))
        .into()
}

/// One distinct focus handle per simultaneously-rendered button. Reusing a
/// handle across two visible buttons aborts GPUI when both request focus in a
/// single frame; each mode owns an indexed handle.
struct ButtonFocus {
    now: FocusHandle,
    paste: FocusHandle,
    copy: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct TimestampsWorkspace {
    input: TextEditor,
    result: TextEditor,
    zone: TextField,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    mode: TimestampMode,
    origin: TimestampOrigin,
    generation: u64,
    session: TimestampsSession,
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

impl TimestampsWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let input = TextEditor::new(window, cx);
        let result = TextEditor::new(window, cx);
        let zone = TextField::new(window, cx);
        zone.assign_text(DEFAULT_ZONE, window, cx);
        let subscriptions = vec![
            input.on_change_in(window, cx, |this, window, cx| {
                if this.suppress_changes {
                    return;
                }
                this.origin = TimestampOrigin::Typed;
                this.schedule(window, cx);
            }),
            zone.on_change_in(window, cx, |this, window, cx| {
                if this.suppress_changes {
                    return;
                }
                this.origin = TimestampOrigin::Typed;
                this.schedule(window, cx);
            }),
        ];
        let history_view = HistoryViewState::load(&history, Timestamps::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Timestamps::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        Self {
            input,
            result,
            zone,
            clipboard,
            history,
            mode: TimestampMode::Auto,
            origin: TimestampOrigin::Typed,
            generation: 0,
            session: TimestampsSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            history_visible: true,
            history_focus: SelectableListFocus::new(),
            choice_focus: SegmentedControlFocus::new(),
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                now: cx.focus_handle().tab_stop(true).tab_index(0),
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

    fn request(&self, cx: &App) -> TimestampsRequest {
        TimestampsRequest {
            input: self.input.text(cx),
            mode: self.mode,
            zone: self.zone.text(cx),
            origin: self.origin,
            generation: self.generation,
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

    /// The explicit Now action reads the system clock, then settles and records
    /// synchronously. The bumped generation keeps two Now actions in the same
    /// second distinct.
    fn use_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let generation = self.generation.wrapping_add(1);
        let request =
            TimestampsRequest::now(&SystemTimestampSource, self.zone.text(cx), generation);
        self.generation = generation;
        self.origin = TimestampOrigin::Now;
        self.mode = request.mode;
        self.suppress_changes = true;
        self.input.assign_text(request.input.clone(), window, cx);
        self.suppress_changes = false;
        let SubmitOutcome::Scheduled(revision) = self.session.submit(request) else {
            return;
        };
        self.session.resolve(revision);
        self.copied = false;
        self.record_settled(cx);
        cx.notify();
    }

    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a Timestamps snapshot serializes");
        let result = self
            .history
            .record(Timestamps::ID, Timestamps::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Timestamps::ID, result);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Timestamps::ID);
        cx.notify();
    }

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            TimestampsEvaluation::Valid {
                representations, ..
            } => {
                self.result
                    .assign_text(representations.display_text(), window, cx);
            }
            _ => self.result.assign_text("", window, cx),
        }
    }

    fn set_mode(&mut self, mode: TimestampMode, window: &mut Window, cx: &mut Context<Self>) {
        self.origin = TimestampOrigin::Typed;
        self.mode = mode;
        self.schedule(window, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.origin = TimestampOrigin::Typed;
            self.input.edit_text(text, window, cx);
        }
    }

    fn copy_result(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.session.evaluation().display_text() {
            self.clipboard.write_text(&text, cx);
            self.copied = true;
            cx.notify();
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = false;
        self.origin = TimestampOrigin::Typed;
        self.suppress_changes = true;
        self.input.edit_text("", window, cx);
        self.suppress_changes = false;
        self.session.clear();
        self.display_epoch = u64::MAX;
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
        snapshot: TimestampsSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mode = snapshot.request.mode;
        let origin = snapshot.request.origin;
        let zone = snapshot.request.zone.clone();
        let input = snapshot.request.input.clone();
        let restored_generation = snapshot.request.generation;
        self.suppress_changes = true;
        self.mode = mode;
        self.origin = origin;
        self.zone.assign_text(zone, window, cx);
        self.input.assign_text(input, window, cx);
        self.session.restore(snapshot);
        // Advance past the restored nonce so the next deliberate Now is always
        // a new revision rather than being deduplicated.
        self.generation = self.generation.max(restored_generation);
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
                .retained(&self.history, Timestamps::ID, &entry)
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
                .retained(&self.history, Timestamps::ID, &entry)
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
                        .map(|snapshot| preview_line(&snapshot.representations))
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
        let actions = Button::with_id("timestamps.history.restore-selected", "Restore selected")
            .disabled(!restore_enabled)
            .focus_handle(self.focus.history_restore.clone())
            .on_click(view_click(cx, |this, window, cx| {
                this.restore_selected(window, cx);
            }));
        let weak = cx.weak_entity();
        let list = SelectableList::new(
            "timestamps.history",
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
            "timestamps.history.restore",
            "Restoring this History entry replaces the current non-empty Timestamps session.",
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

impl Render for TimestampsWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), TimestampsEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.trim().is_empty())
                .unwrap_or(false);

        let modes = SegmentedControl::new(
            "timestamps.mode",
            "Timestamp mode",
            TimestampMode::ALL
                .into_iter()
                .map(|choice| SegmentedOption::new(format!("{choice:?}"), choice.label()))
                .collect(),
            Some(format!("{:?}", self.mode)),
            self.choice_focus.clone(),
        )
        .on_change(Rc::new({
            let weak = cx.weak_entity();
            move |id, window, cx| {
                if let Some(choice) = TimestampMode::ALL
                    .into_iter()
                    .find(|choice| format!("{choice:?}") == id)
                {
                    weak.update(cx, |this, cx| this.set_mode(choice, window, cx))
                        .ok();
                }
            }
        }));

        let toolbar = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(modes)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Timezone"),
                    )
                    .child(div().w_56().child(self.zone.render("timestamps.zone"))),
            )
            .child(
                Button::with_id("timestamps.now", "Now")
                    .focus_handle(self.focus.now.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.use_now(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(copy_feedback(self.copied, "Copied to Clipboard"))
            .child(
                Button::with_id(
                    "timestamps.history.toggle",
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
                Button::with_id("timestamps.paste", "Paste")
                    .focus_handle(self.focus.paste.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::with_id("timestamps.copy-result", "Copy Result")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::with_id("timestamps.clear", "Clear")
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
                "Local Time uses the selected named timezone. Repeated or nonexistent daylight-saving times require an explicit offset.",
            ));

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("Timestamps History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            empty_state("Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), TimestampsEvaluation::Empty) {
            empty_state("Enter a timestamp to convert").into_any_element()
        } else {
            self.result
                .render(true, "timestamps.result")
                .into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(panel(
                "Timestamp Input",
                "Unix, ISO 8601 or local wall time",
                self.input.render(false, "timestamps.input"),
            ))
            .child(panel(
                "Converted Instant",
                "read-only, selectable",
                result_body,
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics())
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<TimestampsSnapshot> {
    if entry.snapshot_version != Timestamps::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

fn preview_line(representations: &TimestampRepresentations) -> String {
    let single_line = format!(
        "{} · {}",
        representations.iso8601, representations.zone_identifier
    );
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

    struct TestRoot(Entity<TimestampsWorkspace>);

    impl Render for TestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
        }
    }

    #[gpui::test]
    fn auto_inference_dst_gap_copy_and_restore_keep_exact_instant(cx: &mut gpui::TestAppContext) {
        cx.update(sofui::init);
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-timestamps-redesign-{}-{}",
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
                .new(|cx| TimestampsWorkspace::new(window, cx, clipboard.clone(), history.clone()));
            captured = Some(view.clone());
            TestRoot(view)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input.edit_text("1700000000", window, cx)
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        let original = workspace.read_with(&cx, |view, cx| view.result.text(cx));
        assert!(original.contains("2023-11-14T22:13:20Z"));
        let entries = history.load(Timestamps::ID).unwrap();
        assert_eq!(entries.len(), 1);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.focus(&workspace.read(cx).focus.copy.clone(), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().as_deref(), Some(original.as_str()));

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let choices = workspace.read(cx).choice_focus.clone();
            window.focus(&choices.handle("timestamps.mode", "Local", cx), cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.zone.edit_text("Europe/Rome", window, cx);
                view.input.edit_text("2026-03-29 02:30:00", window, cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            TimestampMode::Local
        );
        assert!(!workspace.read_with(&cx, |view, _| view
            .session
            .evaluation()
            .is_valid_operation()));
        assert!(workspace
            .read_with(&cx, |view, cx| view.result.text(cx))
            .is_empty());
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.focus(&workspace.read(cx).focus.copy.clone(), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().as_deref(), Some(original.as_str()));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                assert!(view.history_view.select(&entries[0].id));
                view.restore_selected(window, cx);
                assert!(view.history_view.pending_restore.is_some());
                view.confirm_restore(window, cx);
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            TimestampMode::Auto
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.zone.text(cx)),
            "UTC"
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.text(cx)),
            original
        );
        assert_eq!(
            history.load(Timestamps::ID).unwrap().len(),
            1,
            "restore must not record"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
