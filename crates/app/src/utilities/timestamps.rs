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
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    DiagnosticSeverity, HistoryItem, HistoryPanel, TextEditor, TextField, ThemeTokens,
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
    modes: [FocusHandle; 5],
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
        zone.set_text(DEFAULT_ZONE, window, cx);
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
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                modes: std::array::from_fn(|_| cx.focus_handle().tab_stop(true).tab_index(0)),
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
        self.input.set_text(request.input.clone(), window, cx);
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
                    .set_text(representations.display_text(), window, cx);
            }
            _ => self.result.set_text("", window, cx),
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
            self.input.replace_all(text, window, cx);
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
        self.input.replace_all("", window, cx);
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
        self.zone.set_text(zone, window, cx);
        self.input.set_text(input, window, cx);
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
                        .map(|snapshot| preview_line(&snapshot.representations))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn mode_button(&self, mode: TimestampMode, cx: &mut Context<Self>) -> Button {
        Button::new(mode.label())
            .variant(if self.mode == mode {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(self.focus.modes[mode.index()].clone())
            .on_click(view_click(cx, move |this, window, cx| {
                this.set_mode(mode, window, cx);
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
                "Restoring this History entry replaces the current non-empty Timestamps session.",
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

        let mut modes = div().flex().flex_row().items_center().gap_1();
        for mode in TimestampMode::ALL {
            modes = modes.child(self.mode_button(mode, cx));
        }

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
                Button::new("Now")
                    .focus_handle(self.focus.now.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.use_now(window, cx);
                    })),
            )
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
                            .child("Timestamps"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Explicit instants, named zones, local and offline"),
                    ),
            )
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
            .gap_4()
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
