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
    DiagnosticSeverity, HistoryItem, HistoryPanel, TextEditor, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder};
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
    history_entries: Vec<HistoryEntry>,
    history_selected: Option<String>,
    history_visible: bool,
    history_error: Option<String>,
    pending_restore: Option<HistoryEntry>,
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
        let (history_entries, history_error) = match history.load(Jwt::ID) {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
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
            history_entries,
            history_selected: None,
            history_visible: true,
            history_error,
            pending_restore: None,
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
        match self.history.record(Jwt::ID, Jwt::SNAPSHOT_VERSION, payload) {
            Ok(entries) => {
                self.history_entries = entries;
                self.history_error = None;
            }
            Err(error) => {
                self.history_error = Some(error.to_string());
                cx.notify();
            }
        }
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
            self.history_error =
                Some("This History entry uses a snapshot version this build cannot read.".into());
            cx.notify();
            return;
        };
        let current = self.input.text(cx);
        if !current.is_empty() && current != snapshot.request.input {
            self.pending_restore = Some(entry);
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
        self.pending_restore = None;
        self.copied = false;
        self.sync_display(window, cx);
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.pending_restore.clone() {
            if let Some(snapshot) = decode_snapshot(&entry) {
                self.apply_restore(snapshot, window, cx);
            }
        }
    }

    fn cancel_restore(&mut self, cx: &mut Context<Self>) {
        self.pending_restore = None;
        cx.notify();
    }

    fn restore_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected) = self.history_selected.clone() else {
            return;
        };
        if let Some(entry) = self
            .history_entries
            .iter()
            .find(|entry| entry.id == selected)
            .cloned()
        {
            self.request_restore(entry, window, cx);
        }
    }

    fn history_items(&self) -> Vec<HistoryItem> {
        self.history_entries
            .iter()
            .map(|entry| {
                let snapshot = decode_snapshot(entry);
                HistoryItem {
                    id: entry.id.clone(),
                    label: entry.captured_at.clone(),
                    preview: snapshot
                        .as_ref()
                        .map(|snapshot| preview_line(&snapshot.payload))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn render_notice(&self) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
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
        let selected = self.history_selected.clone();
        let restore_enabled = selected
            .as_ref()
            .and_then(|id| self.history_entries.iter().find(|entry| &entry.id == id))
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
            "No retained operations yet. Enable recording to keep entries.",
        )
        .on_select(Rc::new(move |id, _window, cx| {
            weak.update(cx, |this, cx| {
                this.history_selected = Some(id.to_owned());
                cx.notify();
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
            .border_color(ThemeTokens::graphite().border())
            .child(panel)
    }

    fn render_restore_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
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
                "Restoring this History entry replaces the current non-empty JWT Decoder session.",
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

impl Render for JwtWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::graphite();
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
                Button::new(if self.recording_enabled {
                    "Record to History: on"
                } else {
                    "Record to History (off by default)"
                })
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
                Button::new("Copy Header")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy_header.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_header(cx);
                    })),
            )
            .child(
                Button::new("Copy Result")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy_result.clone())
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
                            .child("JWT Decoder"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Local, offline inspection. No keys, no network."),
                    ),
            )
            .child(toolbar)
            .child(self.render_notice());

        if self.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("History is paused for JWT Decoder: {error}"),
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
            .gap_4()
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
