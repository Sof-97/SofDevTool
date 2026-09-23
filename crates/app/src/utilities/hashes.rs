//! The Hashes workspace: choose an algorithm and representation, then
//! explicitly Hash the exact UTF-8 bytes with a fresh Rust History.
//!
//! Hashing is a deliberate action rather than live evaluation. Editing the
//! input, algorithm or representation invalidates the visible digest so no
//! stale result remains; pressing Hash recomputes and records exactly one
//! settled valid operation, including for empty input.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, AnyView, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::hashes::{
    HashAlgorithm, HashRepresentation, Hashes, HashesEvaluation, HashesRequest, HashesSnapshot,
};
use sofdevtool_core::utility::Utility;
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    DiagnosticSeverity, HistoryItem, HistoryPanel, TextEditor, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::workbench::Workbench;

type HashesSession = Session<Hashes>;

/// Builds the Hashes workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| HashesWorkspace::new(window, cx, clipboard, history))
        .into()
}

/// Every simultaneously focusable control owns a distinct handle. Reusing one
/// handle for two rendered buttons aborts GPUI.
struct ButtonFocus {
    sha256: FocusHandle,
    sha384: FocusHandle,
    sha512: FocusHandle,
    sha1: FocusHandle,
    md5: FocusHandle,
    lowercase_hex: FocusHandle,
    uppercase_hex: FocusHandle,
    base64: FocusHandle,
    hash: FocusHandle,
    paste: FocusHandle,
    copy: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct HashesWorkspace {
    input: TextEditor,
    result: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    algorithm: HashAlgorithm,
    representation: HashRepresentation,
    session: HashesSession,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    _history_subscription: HistorySubscription,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl HashesWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let input = TextEditor::new(window, cx);
        let result = TextEditor::new(window, cx);
        let subscriptions = vec![input.on_change_in(window, cx, |this, window, cx| {
            this.invalidate(window, cx);
        })];
        let history_view = HistoryViewState::load(&history, Hashes::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Hashes::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        Self {
            input,
            result,
            clipboard,
            history,
            algorithm: HashAlgorithm::Sha256,
            representation: HashRepresentation::LowercaseHex,
            session: HashesSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            history_visible: true,
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                sha256: cx.focus_handle().tab_stop(true).tab_index(0),
                sha384: cx.focus_handle().tab_stop(true).tab_index(0),
                sha512: cx.focus_handle().tab_stop(true).tab_index(0),
                sha1: cx.focus_handle().tab_stop(true).tab_index(0),
                md5: cx.focus_handle().tab_stop(true).tab_index(0),
                lowercase_hex: cx.focus_handle().tab_stop(true).tab_index(0),
                uppercase_hex: cx.focus_handle().tab_stop(true).tab_index(0),
                base64: cx.focus_handle().tab_stop(true).tab_index(0),
                hash: cx.focus_handle().tab_stop(true).tab_index(0),
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

    fn request(&self, cx: &gpui::App) -> HashesRequest {
        HashesRequest {
            input: self.input.text(cx),
            algorithm: self.algorithm,
            representation: self.representation,
        }
    }

    /// Discards the visible digest after any input, algorithm or representation
    /// change, so a result never appears to describe the current input.
    fn invalidate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.suppress_changes {
            return;
        }
        self.copied = false;
        self.session.clear();
        self.display_epoch = u64::MAX;
        self.sync_display(window, cx);
        cx.notify();
    }

    /// An explicit Hash: a fresh revision is forced so a deliberate repeat
    /// records separately, then resolved and recorded synchronously.
    fn hash(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = false;
        self.session.clear();
        let SubmitOutcome::Scheduled(revision) = self.session.submit(self.request(cx)) else {
            return;
        };
        self.session.resolve(revision);
        self.display_epoch = u64::MAX;
        self.sync_display(window, cx);
        self.record_settled(cx);
        cx.notify();
    }

    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a Hashes snapshot serializes");
        let result = self
            .history
            .record(Hashes::ID, Hashes::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Hashes::ID, result);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Hashes::ID);
        cx.notify();
    }

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            HashesEvaluation::Valid { output } => {
                self.result.set_text(output.clone(), window, cx);
            }
            _ => self.result.set_text("", window, cx),
        }
    }

    fn set_algorithm(
        &mut self,
        algorithm: HashAlgorithm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.algorithm == algorithm {
            return;
        }
        self.algorithm = algorithm;
        self.invalidate(window, cx);
    }

    fn set_representation(
        &mut self,
        representation: HashRepresentation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.representation == representation {
            return;
        }
        self.representation = representation;
        self.invalidate(window, cx);
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
        snapshot: HashesSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.algorithm = snapshot.request.algorithm;
        self.representation = snapshot.request.representation;
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
                .retained(&self.history, Hashes::ID, &entry)
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
                .retained(&self.history, Hashes::ID, &entry)
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
                        .map(|snapshot| {
                            format!(
                                "{} · {}",
                                snapshot.request.algorithm.label(),
                                preview_line(&snapshot.output)
                            )
                        })
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn algorithm_button(&self, algorithm: HashAlgorithm, cx: &mut Context<Self>) -> Button {
        Button::new(algorithm.label())
            .variant(if self.algorithm == algorithm {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(match algorithm {
                HashAlgorithm::Sha256 => self.focus.sha256.clone(),
                HashAlgorithm::Sha384 => self.focus.sha384.clone(),
                HashAlgorithm::Sha512 => self.focus.sha512.clone(),
                HashAlgorithm::Sha1 => self.focus.sha1.clone(),
                HashAlgorithm::Md5 => self.focus.md5.clone(),
            })
            .on_click(view_click(cx, move |this, window, cx| {
                this.set_algorithm(algorithm, window, cx);
            }))
    }

    fn representation_button(
        &self,
        representation: HashRepresentation,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(representation.label())
            .variant(if self.representation == representation {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(match representation {
                HashRepresentation::LowercaseHex => self.focus.lowercase_hex.clone(),
                HashRepresentation::UppercaseHex => self.focus.uppercase_hex.clone(),
                HashRepresentation::Base64 => self.focus.base64.clone(),
            })
            .on_click(view_click(cx, move |this, window, cx| {
                this.set_representation(representation, window, cx);
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
                "Restoring this History entry replaces the current non-empty Hashes session.",
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

    fn control_label(&self, text: &'static str) -> impl IntoElement {
        div()
            .text_xs()
            .text_color(ThemeTokens::active().text_muted())
            .child(text)
    }
}

impl Render for HashesWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
        let can_copy = self.session.evaluation().is_valid_operation();

        let mut algorithm_row = div().flex().flex_row().items_center().gap_2().flex_wrap();
        for algorithm in HashAlgorithm::ALL {
            algorithm_row = algorithm_row.child(self.algorithm_button(algorithm, cx));
        }
        let mut representation_row = div().flex().flex_row().items_center().gap_2().flex_wrap();
        for representation in HashRepresentation::ALL {
            representation_row =
                representation_row.child(self.representation_button(representation, cx));
        }

        let algorithm_toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(self.control_label("Algorithm"))
            .child(algorithm_row)
            .child(div().w_3())
            .child(self.control_label("Output"))
            .child(representation_row)
            .child(div().w_3())
            .child(
                Button::new("Hash")
                    .variant(ButtonVariant::Primary)
                    .focus_handle(self.focus.hash.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.hash(window, cx);
                    })),
            );

        let actions_toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
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
                            .child("Hashes"),
                    )
                    .child(div().text_xs().text_color(tokens.text_muted()).child(
                        "Hashes the exact UTF-8 bytes. Files, HMAC, and password hashing are not included.",
                    )),
            )
            .child(algorithm_toolbar)
            .child(actions_toolbar);

        if self.algorithm.is_legacy() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                self.algorithm.notice(),
                None,
            ));
        }
        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("Hashes History: {error}"),
                None,
            ));
        }

        let result_body = if matches!(self.session.evaluation(), HashesEvaluation::Valid { .. }) {
            self.result.render(true, "hashes.result").into_any_element()
        } else {
            empty_state("Press Hash to compute the digest").into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_4()
            .flex_1()
            .min_h_0()
            .child(panel(
                "UTF-8 Input",
                "exact bytes",
                self.input.render(false, "hashes.input"),
            ))
            .child(panel("Digest", "read-only, selectable", result_body));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics())
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<HashesSnapshot> {
    if entry.snapshot_version != Hashes::SNAPSHOT_VERSION {
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
