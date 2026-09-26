//! The Timestamps workspace: explicit Auto/manual inference, a named-zone
//! control, an injected-Now action, synchronized read-only representations and
//! fresh Rust History. Restoring never reads the clock again.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, Entity, IntoElement, Render, Subscription, Window};
use gpui_kit::component::{
    button::Button,
    input::{Input, InputEvent, InputState, TextareaState},
    list::ListState,
    tab::{Tab, TabBar},
    ActiveTheme as _, Disableable as _,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::timestamps::{
    SystemTimestampSource, TimestampMode, TimestampOrigin, TimestampRepresentations, Timestamps,
    TimestampsEvaluation, TimestampsRequest, TimestampsSnapshot,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::ui;
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

pub struct TimestampsWorkspace {
    input: Entity<TextareaState>,
    result: Entity<TextareaState>,
    zone: Entity<InputState>,
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
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl TimestampsWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let input = cx.new(|cx| TextareaState::new(window, cx));
        let result = cx.new(|cx| TextareaState::new(window, cx));
        let zone = cx.new(|cx| InputState::new(window, cx).default_value(DEFAULT_ZONE));
        let subscriptions = vec![
            cx.subscribe_in(
                &input,
                window,
                |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        if this.suppress_changes {
                            return;
                        }
                        this.origin = TimestampOrigin::Typed;
                        this.schedule(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &zone,
                window,
                |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        if this.suppress_changes {
                            return;
                        }
                        this.origin = TimestampOrigin::Typed;
                        this.schedule(window, cx);
                    }
                },
            ),
        ];
        let history_view = HistoryViewState::load(&history, Timestamps::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Timestamps::ID, move |cx| {
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
            history_list,
            _history_subscription: history_subscription,
            _subscriptions: subscriptions,
        };
        workspace.sync_history(cx);
        workspace
    }

    fn request(&self, cx: &App) -> TimestampsRequest {
        TimestampsRequest {
            input: self.input.read(cx).value().to_string(),
            mode: self.mode,
            zone: self.zone.read(cx).value().to_string(),
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
        let request = TimestampsRequest::now(
            &SystemTimestampSource,
            self.zone.read(cx).value().to_string(),
            generation,
        );
        self.generation = generation;
        self.origin = TimestampOrigin::Now;
        self.mode = request.mode;
        self.suppress_changes = true;
        self.input.update(cx, |state, cx| {
            state.set_value(request.input.clone(), window, cx)
        });
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
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Timestamps::ID);
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
        match self.session.evaluation() {
            TimestampsEvaluation::Valid {
                representations, ..
            } => {
                self.result.update(cx, |state, cx| {
                    state.set_value(representations.display_text(), window, cx)
                });
            }
            _ => self
                .result
                .update(cx, |state, cx| state.set_value("", window, cx)),
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
            self.input
                .update(cx, |state, cx| state.replace_all(text, window, cx));
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
        self.input
            .update(cx, |state, cx| state.replace_all("", window, cx));
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
        let current = self.input.read(cx).value().to_string();
        if !current.is_empty() && current != snapshot.request.input {
            self.history_view.pending_restore = Some(entry);
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current non-empty Timestamps session.",
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
        self.zone
            .update(cx, |state, cx| state.set_value(zone, window, cx));
        self.input
            .update(cx, |state, cx| state.set_value(input, window, cx));
        self.session.restore(snapshot);
        // Advance past the restored nonce so the next deliberate Now is always
        // a new revision rather than being deduplicated.
        self.generation = self.generation.max(restored_generation);
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
                        .map(|snapshot| preview_line(&snapshot.representations))
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
        let theme = cx.theme().clone();
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
        let restore = Button::new("timestamps.history.restore-selected")
            .label("Restore selected")
            .disabled(!restore_enabled)
            .on_click(cx.listener(|this, _event, window, cx| this.restore_selected(window, cx)));
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

impl Render for TimestampsWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let theme = cx.theme().clone();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), TimestampsEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.trim().is_empty())
                .unwrap_or(false);

        let selected_mode = TimestampMode::ALL
            .iter()
            .position(|choice| *choice == self.mode)
            .unwrap_or(0);
        let modes = TabBar::new("timestamps.mode")
            .segmented()
            .selected_index(selected_mode)
            .children(
                TimestampMode::ALL
                    .into_iter()
                    .map(|choice| Tab::new().label(choice.label())),
            )
            .on_click(cx.listener(|this, choice, window, cx| {
                if let Some(mode) = TimestampMode::ALL.get(*choice) {
                    this.set_mode(*mode, window, cx);
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
                            .text_color(theme.muted_foreground)
                            .child("Timezone"),
                    )
                    .child(
                        div().w_56().child(
                            Input::new(&self.zone)
                                .accessibility_id("timestamps.zone")
                                .w_full(),
                        ),
                    ),
            )
            .child(
                Button::new("timestamps.now")
                    .label("Now")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.use_now(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(ui::copy_feedback(cx, self.copied, "Copied to Clipboard"))
            .child(
                Button::new("timestamps.history.toggle")
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
                Button::new("timestamps.paste")
                    .label("Paste")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::new("timestamps.copy-result")
                    .label("Copy Result")
                    .disabled(!can_copy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::new("timestamps.clear")
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
            .child(toolbar)
            .child(div().text_xs().text_color(theme.muted_foreground).child(
                "Local Time uses the selected named timezone. Repeated or nonexistent daylight-saving times require an explicit offset.",
            ));

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Timestamps History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            ui::empty_state(cx, "Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), TimestampsEvaluation::Empty) {
            ui::empty_state(cx, "Enter a timestamp to convert").into_any_element()
        } else {
            ui::multiline_editor(&self.result, true, "timestamps.result").into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(ui::panel(
                cx,
                "Timestamp Input",
                "Unix, ISO 8601 or local wall time",
                ui::multiline_editor(&self.input, false, "timestamps.input"),
            ))
            .child(ui::panel(
                cx,
                "Converted Instant",
                "read-only, selectable",
                result_body,
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics(cx))
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
    fn auto_inference_dst_gap_copy_and_restore_keep_exact_instant(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_kit::init);
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
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input
                    .update(cx, |state, cx| state.replace_all("1700000000", window, cx))
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        let original =
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string());
        assert!(original.contains("2023-11-14T22:13:20Z"));
        let entries = history.load(Timestamps::ID).unwrap();
        assert_eq!(entries.len(), 1);
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_result(cx));
        });
        assert_eq!(clipboard.0.borrow().as_deref(), Some(original.as_str()));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.set_mode(TimestampMode::Local, window, cx);
                view.zone
                    .update(cx, |state, cx| state.replace_all("Europe/Rome", window, cx));
                view.input.update(cx, |state, cx| {
                    state.replace_all("2026-03-29 02:30:00", window, cx)
                });
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
            .read_with(&cx, |view, cx| view.result.read(cx).value().to_string())
            .is_empty());
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_result(cx));
        });
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
            workspace.read_with(&cx, |view, cx| view.zone.read(cx).value().to_string()),
            "UTC"
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
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
