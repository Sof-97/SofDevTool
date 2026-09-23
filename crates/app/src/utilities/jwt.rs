//! The JWT Decoder workspace: decode the header and payload as readable JSON,
//! keep the signature opaque, and record to History only after the user
//! explicitly opts in.
//!
//! History is off by default for this Utility. Nothing is recorded unless the
//! local "Record to History" toggle is on; invalid and intermediate evaluations
//! never persist. This workspace never verifies the signature, interprets
//! claims or decides trust, and it exposes no key input.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::jwt::{
    Jwt, JwtEvaluation, JwtRequest, JwtSnapshot, JWT_NO_VERIFICATION_NOTICE,
};
use sofdevtool_core::utility::Utility;
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    ConfirmationBar, DiagnosticSeverity, SelectableList, SelectableListFocus, SelectableRow,
    TextEditor, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(200);

type JwtSession = Session<Jwt>;

/// Builds the JWT Decoder workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| JwtWorkspace::new(window, cx, clipboard, history))
        .into()
}

/// Every simultaneously focusable control owns a distinct handle. Reusing one
/// handle for two rendered buttons aborts GPUI.
struct ButtonFocus {
    record: FocusHandle,
    history_toggle: FocusHandle,
    paste: FocusHandle,
    copy_header: FocusHandle,
    copy_result: FocusHandle,
    clear: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct JwtWorkspace {
    input: TextEditor,
    header: TextEditor,
    payload: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    session: JwtSession,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    /// Off by default: this Utility records nothing until the user opts in.
    recording_enabled: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    history_focus: SelectableListFocus,
    _history_subscription: HistorySubscription,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl JwtWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let input = TextEditor::new(window, cx);
        let header = TextEditor::new(window, cx);
        let payload = TextEditor::new(window, cx);
        let subscriptions = vec![input.on_change_in(window, cx, |this, window, cx| {
            this.schedule(window, cx);
        })];
        let history_view = HistoryViewState::load(&history, Jwt::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Jwt::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        let recording_enabled = history.is_recording(Jwt::ID);
        Self {
            input,
            header,
            payload,
            clipboard,
            history,
            session: JwtSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            recording_enabled,
            history_view,
            history_visible: true,
            history_focus: SelectableListFocus::new(),
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                record: cx.focus_handle().tab_stop(true).tab_index(0),
                history_toggle: cx.focus_handle().tab_stop(true).tab_index(0),
                paste: cx.focus_handle().tab_stop(true).tab_index(0),
                copy_header: cx.focus_handle().tab_stop(true).tab_index(0),
                copy_result: cx.focus_handle().tab_stop(true).tab_index(0),
                clear: cx.focus_handle().tab_stop(true).tab_index(0),
                history_restore: cx.focus_handle().tab_stop(true).tab_index(0),
                history_confirm: cx.focus_handle().tab_stop(true).tab_index(0),
                history_cancel: cx.focus_handle().tab_stop(true).tab_index(0),
            },
            _subscriptions: subscriptions,
        }
    }

    fn request(&self, cx: &App) -> JwtRequest {
        JwtRequest {
            input: self.input.text(cx),
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
        if !may_record(self.recording_enabled, self.session.evaluation()) {
            return;
        }
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a JWT snapshot serializes");
        let result = self.history.record(Jwt::ID, Jwt::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Jwt::ID, result);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Jwt::ID);
        cx.notify();
    }

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            JwtEvaluation::Valid {
                header, payload, ..
            } => {
                self.header.set_text(header.clone(), window, cx);
                self.payload.set_text(payload.clone(), window, cx);
            }
            _ => {
                self.header.set_text("", window, cx);
                self.payload.set_text("", window, cx);
            }
        }
    }

    fn toggle_recording(&mut self, cx: &mut Context<Self>) {
        self.recording_enabled = !self.recording_enabled;
        self.history
            .set_utility_enabled(Jwt::ID, self.recording_enabled);
        cx.notify();
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.input.replace_all(text, window, cx);
        }
    }

    fn copy_header(&mut self, cx: &mut Context<Self>) {
        if let Some(header) = self.session.evaluation().header() {
            let header = header.to_owned();
            self.clipboard.write_text(&header, cx);
            self.copied = true;
            cx.notify();
        }
    }

    fn copy_result(&mut self, cx: &mut Context<Self>) {
        if let Some(payload) = self.session.evaluation().payload() {
            let payload = payload.to_owned();
            self.clipboard.write_text(&payload, cx);
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
        snapshot: JwtSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
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
            if self.history_view.retained(&self.history, Jwt::ID, &entry) {
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
            if self.history_view.retained(&self.history, Jwt::ID, &entry) {
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
                        .map(|snapshot| preview_line(&snapshot.payload))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: (!snapshot.is_some()).then(|| "Unavailable".to_owned()),
                    selectable: true,
                }
            })
            .collect()
    }

    fn render_notice(&self) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(tokens.warning())
            .bg(tokens.surface_raised())
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(tokens.warning())
                    .child("No signature or claim verification"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text())
                    .child(JWT_NO_VERIFICATION_NOTICE),
            )
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
        let actions = Button::with_id("jwt-decoder.history.restore-selected", "Restore selected")
            .disabled(!restore_enabled)
            .focus_handle(self.focus.history_restore.clone())
            .on_click(view_click(cx, |this, window, cx| {
                this.restore_selected(window, cx)
            }));
        let weak = cx.weak_entity();
        let list = SelectableList::new(
            "jwt-decoder.history",
            "History",
            self.history_items(),
            selected,
            "No retained operations yet. Enable recording to keep entries.",
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
            "jwt-decoder.history.restore",
            "Restoring this History entry replaces the current non-empty JWT Decoder session.",
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

impl Render for JwtWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
        let can_copy = self.session.evaluation().is_valid_operation();
        let signature = self
            .session
            .evaluation()
            .signature()
            .unwrap_or("—")
            .to_owned();

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(
                Button::with_id(
                    "jwt-decoder.recording",
                    if self.recording_enabled {
                        "Record to History: on"
                    } else {
                        "Record to History (off by default)"
                    },
                )
                .variant(if self.recording_enabled {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                })
                .focus_handle(self.focus.record.clone())
                .on_click(view_click(cx, |this, _window, cx| {
                    this.toggle_recording(cx);
                })),
            )
            .child(div().flex_1())
            .child(copy_feedback(self.copied, "Copied to Clipboard"))
            .child(
                Button::with_id(
                    "jwt-decoder.history.toggle",
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
                Button::with_id("jwt-decoder.paste", "Paste")
                    .focus_handle(self.focus.paste.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::with_id("jwt-decoder.copy-header", "Copy Header")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy_header.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_header(cx);
                    })),
            )
            .child(
                Button::with_id("jwt-decoder.copy-result", "Copy Result")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy_result.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::with_id("jwt-decoder.clear", "Clear")
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
            .child(self.render_notice());

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("JWT Decoder History: {error}"),
                None,
            ));
        }

        let (header_body, payload_body) = if can_copy {
            (
                self.header
                    .render(true, "jwt-decoder.header")
                    .into_any_element(),
                self.payload
                    .render(true, "jwt-decoder.payload")
                    .into_any_element(),
            )
        } else {
            (
                empty_state("Decoded header appears here").into_any_element(),
                empty_state("Decoded payload appears here").into_any_element(),
            )
        };

        let results = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap_4()
            .child(panel("Header JSON", "read-only, unverified", header_body))
            .child(panel("Payload JSON", "read-only, unverified", payload_body));

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(panel(
                "JWT",
                "exact three dot-separated segments",
                self.input.render(false, "jwt-decoder.input"),
            ))
            .child(results);
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }

        column
            .child(workspace)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap_2()
                    .w_full()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(tokens.text_muted())
                            .child("Signature (opaque, unverified):"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_color(tokens.text())
                            .child(signature),
                    ),
            )
            .child(self.render_diagnostics())
    }
}

/// Recording is opt-in for the JWT Decoder and only ever retains a settled
/// valid operation. Invalid and neutral evaluations never persist.
fn may_record(recording_enabled: bool, evaluation: &JwtEvaluation) -> bool {
    recording_enabled && evaluation.is_valid_operation()
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<JwtSnapshot> {
    if entry.snapshot_version != Jwt::SNAPSHOT_VERSION {
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
        "Empty payload".to_owned()
    } else {
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sofdevtool_core::utilities::jwt::evaluate;

    const TOKEN: &str = concat!(
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
        ".eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiYWRtaW4iOnRydWUsImlhdCI6MTUxNjIzOTAyMn0",
        ".SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
    );

    #[test]
    fn recording_is_opt_in_by_default() {
        let valid = evaluate(&JwtRequest::new(TOKEN));
        assert!(valid.is_valid_operation());
        assert!(
            !may_record(false, &valid),
            "the fresh default must not record"
        );
        assert!(
            may_record(true, &valid),
            "explicit opt-in records a valid operation"
        );
    }

    #[test]
    fn invalid_and_neutral_operations_never_record() {
        assert!(!may_record(true, &JwtEvaluation::Empty));
        assert!(!may_record(true, &evaluate(&JwtRequest::new("only.two"))));
    }

    #[test]
    fn history_snapshot_decoding_rejects_unknown_versions() {
        let snapshot = JwtSnapshot {
            request: JwtRequest::new(TOKEN),
            header: "{}".to_owned(),
            payload: "{}".to_owned(),
            signature: "opaque".to_owned(),
        };
        let entry = HistoryEntry {
            id: "1".to_owned(),
            captured_at: "2026-01-01T00:00:00Z".to_owned(),
            utility_id: Jwt::ID.to_owned(),
            snapshot_version: Jwt::SNAPSHOT_VERSION,
            payload: serde_json::to_value(&snapshot).expect("snapshot serializes"),
        };
        assert!(decode_snapshot(&entry).is_some());
        let unknown = HistoryEntry {
            snapshot_version: 99,
            ..entry
        };
        assert!(decode_snapshot(&unknown).is_none());
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use gpui::{Entity, VisualTestContext};

    use crate::history::{HistoryPolicy, HistoryStore, SystemClock};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
    const TOKEN: &str = concat!(
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
        ".eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiYWRtaW4iOnRydWUsImlhdCI6MTUxNjIzOTAyMn0",
        ".SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
    );

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

    struct TestRoot(Entity<JwtWorkspace>);

    impl Render for TestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
        }
    }

    #[gpui::test]
    fn valid_copy_invalid_state_and_opt_in_history_use_live_controls(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(sofdevtool_ui::init);
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-jwt-redesign-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut policy = HistoryPolicy::default();
        policy.defaults.insert(Jwt::ID.to_owned(), false);
        let history = Rc::new(HistoryRecorder::with_policy(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
            policy,
            None,
        ));
        let clipboard = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| JwtWorkspace::new(window, cx, clipboard.clone(), history.clone()));
            captured = Some(view.clone());
            TestRoot(view)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.input.edit_text(TOKEN, window, cx));
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert!(workspace.read_with(&cx, |view, _| view
            .session
            .evaluation()
            .is_valid_operation()));
        assert!(history.load(Jwt::ID).unwrap().is_empty());
        let header = workspace.read_with(&cx, |view, cx| view.header.text(cx));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.copy_header.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().as_deref(), Some(header.as_str()));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.input.edit_text("only.two", window, cx));
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
            let focus = workspace.read(cx).focus.copy_result.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().as_deref(), Some(header.as_str()));

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.record.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert!(workspace.read_with(&cx, |view, _| view.recording_enabled));
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.input.edit_text(TOKEN, window, cx));
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(Jwt::ID).unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
