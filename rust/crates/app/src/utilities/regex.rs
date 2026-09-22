//! The Regex workspace: test a Rust `regex` crate pattern against text, inspect
//! every match and capture, and preview a replacement with the actual dialect
//! and replacement syntax shown in the UI.
//!
//! This workspace never emulates ICU: unsupported look-around and
//! backreferences surface as diagnostics. Evaluation is bounded by the named
//! limits in the core contract, debounced off the UI thread and revision-gated
//! through the shared [`Session`], so only the current revision publishes a
//! result or a History snapshot.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::regex::{
    CaptureInfo, Regex, RegexEvaluation, RegexFlags, RegexRequest, RegexSnapshot,
    RUST_REGEX_DIALECT_NOTE, RUST_REGEX_ENGINE_LABEL, RUST_REGEX_FLAGS_NOTE,
    RUST_REGEX_REPLACEMENT_NOTE,
};
use sofdevtool_core::utility::Utility;
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    DiagnosticSeverity, HistoryItem, HistoryPanel, TextEditor, TextField, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder};
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(200);

type RegexSession = Session<Regex>;

/// One boolean flag this workspace can set on the Rust regex engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flag {
    CaseInsensitive,
    MultiLine,
    DotMatchesNewLine,
    IgnoreWhitespace,
    SwapGreed,
    Crlf,
}

impl Flag {
    const ALL: [Flag; 6] = [
        Flag::CaseInsensitive,
        Flag::MultiLine,
        Flag::DotMatchesNewLine,
        Flag::IgnoreWhitespace,
        Flag::SwapGreed,
        Flag::Crlf,
    ];

    fn index(self) -> usize {
        match self {
            Flag::CaseInsensitive => 0,
            Flag::MultiLine => 1,
            Flag::DotMatchesNewLine => 2,
            Flag::IgnoreWhitespace => 3,
            Flag::SwapGreed => 4,
            Flag::Crlf => 5,
        }
    }

    /// A distinct, mnemonic label. Distinct labels also keep each [`Button`]'s
    /// element id unique.
    fn label(self) -> &'static str {
        match self {
            Flag::CaseInsensitive => "i  case-insensitive",
            Flag::MultiLine => "m  multi-line",
            Flag::DotMatchesNewLine => "s  dot matches newline",
            Flag::IgnoreWhitespace => "x  ignore whitespace",
            Flag::SwapGreed => "U  swap greed",
            Flag::Crlf => "R  CRLF mode",
        }
    }

    fn enabled(self, flags: RegexFlags) -> bool {
        match self {
            Flag::CaseInsensitive => flags.case_insensitive,
            Flag::MultiLine => flags.multi_line,
            Flag::DotMatchesNewLine => flags.dot_matches_new_line,
            Flag::IgnoreWhitespace => flags.ignore_whitespace,
            Flag::SwapGreed => flags.swap_greed,
            Flag::Crlf => flags.crlf,
        }
    }

    fn toggled(self, mut flags: RegexFlags) -> RegexFlags {
        match self {
            Flag::CaseInsensitive => flags.case_insensitive = !flags.case_insensitive,
            Flag::MultiLine => flags.multi_line = !flags.multi_line,
            Flag::DotMatchesNewLine => flags.dot_matches_new_line = !flags.dot_matches_new_line,
            Flag::IgnoreWhitespace => flags.ignore_whitespace = !flags.ignore_whitespace,
            Flag::SwapGreed => flags.swap_greed = !flags.swap_greed,
            Flag::Crlf => flags.crlf = !flags.crlf,
        }
        flags
    }
}

/// Builds the Regex workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| RegexWorkspace::new(window, cx, clipboard, history))
        .into()
}

/// Every simultaneously focusable control owns a distinct handle. Reusing a
/// handle for two rendered buttons aborts GPUI.
struct ButtonFocus {
    flags: [FocusHandle; 6],
    paste: FocusHandle,
    copy: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct RegexWorkspace {
    pattern: TextField,
    text: TextEditor,
    replacement: TextField,
    preview: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    flags: RegexFlags,
    session: RegexSession,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_entries: Vec<HistoryEntry>,
    history_selected: Option<String>,
    history_visible: bool,
    history_error: Option<String>,
    pending_restore: Option<HistoryEntry>,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl RegexWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let pattern = TextField::new(window, cx);
        let text = TextEditor::new(window, cx);
        let replacement = TextField::new(window, cx);
        let preview = TextEditor::new(window, cx);
        let subscriptions = vec![
            pattern.on_change_in(window, cx, |this, window, cx| {
                this.schedule(window, cx);
            }),
            text.on_change_in(window, cx, |this, window, cx| {
                this.schedule(window, cx);
            }),
            replacement.on_change_in(window, cx, |this, window, cx| {
                this.schedule(window, cx);
            }),
        ];
        let (history_entries, history_error) = match history.load(Regex::ID) {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        Self {
            pattern,
            text,
            replacement,
            preview,
            clipboard,
            history,
            flags: RegexFlags::default(),
            session: RegexSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_entries,
            history_selected: None,
            history_visible: true,
            history_error,
            pending_restore: None,
            focus: ButtonFocus {
                flags: std::array::from_fn(|_| cx.focus_handle().tab_stop(true).tab_index(0)),
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

    fn request(&self, cx: &App) -> RegexRequest {
        RegexRequest {
            pattern: self.pattern.text(cx),
            text: self.text.text(cx),
            flags: self.flags,
            replacement: self.replacement.text(cx),
        }
    }

    /// Submits the current request. A changed request clears the visible result
    /// immediately and schedules a debounced, revision-gated evaluation. The
    /// debounce wait runs on the background executor, and a stale revision can
    /// never settle because the shared session rejects it.
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

    /// Records the one settled valid operation for the current revision, if any.
    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a Regex snapshot serializes");
        match self
            .history
            .record(Regex::ID, Regex::SNAPSHOT_VERSION, payload)
        {
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

    /// Mirrors the settled replacement preview into the read-only editor, but
    /// only when the evaluation epoch changed. Obsolete results are never shown.
    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            RegexEvaluation::Valid { replacement, .. } => {
                self.preview
                    .set_text(replacement.clone().unwrap_or_default(), window, cx);
            }
            _ => self.preview.set_text("", window, cx),
        }
    }

    fn toggle_flag(&mut self, flag: Flag, window: &mut Window, cx: &mut Context<Self>) {
        self.flags = flag.toggled(self.flags);
        self.schedule(window, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.text.replace_all(text, window, cx);
        }
    }

    fn copy_result(&mut self, cx: &mut Context<Self>) {
        if let Some(replacement) = self.session.evaluation().replacement() {
            let replacement = replacement.to_owned();
            self.clipboard.write_text(&replacement, cx);
            self.copied = true;
            cx.notify();
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = false;
        self.suppress_changes = true;
        self.pattern.replace_all("", window, cx);
        self.text.replace_all("", window, cx);
        self.replacement.replace_all("", window, cx);
        self.suppress_changes = false;
        self.session.clear();
        self.display_epoch = u64::MAX;
        self.copied = false;
        self.sync_display(window, cx);
        cx.notify();
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
        let current = self.request(cx);
        let non_empty = !(current.pattern.is_empty()
            && current.text.is_empty()
            && current.replacement.is_empty());
        if non_empty && current != snapshot.request {
            self.pending_restore = Some(entry);
            cx.notify();
        } else {
            self.apply_restore(snapshot, window, cx);
        }
    }

    fn apply_restore(
        &mut self,
        snapshot: RegexSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.pattern
            .set_text(snapshot.request.pattern.clone(), window, cx);
        self.text
            .set_text(snapshot.request.text.clone(), window, cx);
        self.replacement
            .set_text(snapshot.request.replacement.clone(), window, cx);
        self.flags = snapshot.request.flags;
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
                        .map(|snapshot| {
                            format!(
                                "{} match(es) · {}",
                                snapshot.matches.len(),
                                preview_line(&snapshot.replacement)
                            )
                        })
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn flag_button(&self, flag: Flag, cx: &mut Context<Self>) -> Button {
        Button::new(flag.label())
            .variant(if flag.enabled(self.flags) {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(self.focus.flags[flag.index()].clone())
            .on_click(view_click(cx, move |this, window, cx| {
                this.toggle_flag(flag, window, cx);
            }))
    }

    fn render_matches(&self) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let mut list = div()
            .id("regex.matches-list")
            .flex()
            .flex_col()
            .gap_2()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll();
        for matched in self.session.evaluation().matches() {
            let mut row = div()
                .flex()
                .flex_col()
                .gap_1()
                .w_full()
                .px_2()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(tokens.border())
                .bg(tokens.surface_raised())
                .child(div().text_sm().text_color(tokens.text()).child(format!(
                    "{}. {}",
                    matched.ordinal + 1,
                    escaped(&matched.value)
                )))
                .child(
                    div()
                        .text_xs()
                        .text_color(tokens.text_muted())
                        .child(format!(
                            "bytes {}..{}",
                            matched.range.start, matched.range.end
                        )),
                );
            for capture in &matched.captures {
                row = row.child(
                    div()
                        .text_xs()
                        .text_color(tokens.text_muted())
                        .child(capture_line(capture)),
                );
            }
            list = list.child(row);
        }
        list
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
            "No retained operations yet.",
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
                "Restoring this History entry replaces the current non-empty Regex session.",
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

impl Render for RegexWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
        let can_copy = self.session.evaluation().is_valid_operation();
        let neutral = matches!(self.session.evaluation(), RegexEvaluation::Empty);
        let pending = neutral
            && self
                .session
                .request()
                .map(|request| !(request.pattern.is_empty() && request.text.is_empty()))
                .unwrap_or(false);

        let mut flags_row = div().flex().flex_row().items_center().flex_wrap().gap_2();
        for flag in Flag::ALL {
            flags_row = flags_row.child(self.flag_button(flag, cx));
        }

        let pattern_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w_24()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Pattern"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(self.pattern.render("regex.pattern")),
            );

        let replacement_row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w_24()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Replacement template"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(self.replacement.render("regex.replacement")),
            );

        let guidance = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(tokens.accent())
                    .child(RUST_REGEX_ENGINE_LABEL),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child(RUST_REGEX_DIALECT_NOTE),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child(RUST_REGEX_FLAGS_NOTE),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child(RUST_REGEX_REPLACEMENT_NOTE),
            );

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .flex_wrap()
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
                            .child("Regex"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Local, offline. No ICU emulation."),
                    ),
            )
            .child(guidance)
            .child(flags_row)
            .child(pattern_row)
            .child(replacement_row)
            .child(toolbar);

        if self.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("History is paused for Regex: {error}"),
                None,
            ));
        }

        let preview_body = if pending {
            empty_state("Evaluating…").into_any_element()
        } else if neutral {
            empty_state("Enter a pattern and test text to begin").into_any_element()
        } else {
            self.preview
                .render(true, "regex.replacement-preview")
                .into_any_element()
        };

        let matches_body = if pending {
            empty_state("Evaluating…").into_any_element()
        } else {
            match self.session.evaluation() {
                RegexEvaluation::Empty => {
                    empty_state("Enter a pattern and test text to begin").into_any_element()
                }
                RegexEvaluation::Valid { matches, .. } if matches.is_empty() => {
                    empty_state("No matches").into_any_element()
                }
                RegexEvaluation::Valid { .. } => self.render_matches().into_any_element(),
                RegexEvaluation::Invalid { .. } => {
                    empty_state("No result was published for the current input").into_any_element()
                }
            }
        };

        let left = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap_4()
            .child(panel(
                "Test Text",
                "exact UTF-8 bytes",
                self.text.render(false, "regex.text"),
            ))
            .child(panel(
                "Replacement Preview",
                "read-only, copyable",
                preview_body,
            ));

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_4()
            .flex_1()
            .min_h_0()
            .child(left)
            .child(panel(
                "Matches & Captures",
                RUST_REGEX_ENGINE_LABEL,
                matches_body,
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics())
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<RegexSnapshot> {
    if entry.snapshot_version != Regex::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

/// One display line for a capture: `$name (\$index) = value [start..end)`, or
/// `(unmatched)` when the group did not participate.
fn capture_line(capture: &CaptureInfo) -> String {
    let name = match &capture.name {
        Some(name) => format!("{name} (${})", capture.index),
        None => format!("${}", capture.index),
    };
    match (&capture.value, capture.range) {
        (Some(value), Some(range)) => {
            format!(
                "  {name} = {} [{}..{})",
                escaped(value),
                range.start,
                range.end
            )
        }
        _ => format!("  {name} = (unmatched)"),
    }
}

/// Renders an exact value on one line; control characters stay visible.
fn escaped(text: &str) -> String {
    if text.is_empty() {
        return "(empty)".to_owned();
    }
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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
mod tests {
    use super::*;
    use sofdevtool_core::utilities::regex::evaluate;

    #[test]
    fn history_snapshot_decoding_rejects_unknown_versions() {
        let request = RegexRequest::new(r"\w+", "one two");
        let evaluation = evaluate(&request);
        let snapshot =
            <Regex as Utility>::snapshot(&request, &evaluation).expect("valid operation snapshots");
        let entry = HistoryEntry {
            id: "1".to_owned(),
            captured_at: "2026-01-01T00:00:00Z".to_owned(),
            utility_id: Regex::ID.to_owned(),
            snapshot_version: Regex::SNAPSHOT_VERSION,
            payload: serde_json::to_value(&snapshot).expect("snapshot serializes"),
        };
        assert!(decode_snapshot(&entry).is_some());
        let unknown = HistoryEntry {
            snapshot_version: 99,
            ..entry
        };
        assert!(decode_snapshot(&unknown).is_none());
    }

    #[test]
    fn invalid_operations_do_not_produce_a_replaceable_snapshot() {
        let request = RegexRequest::new(r"a(?=b)", "ab");
        let evaluation = evaluate(&request);
        assert!(!evaluation.is_valid_operation());
        assert!(<Regex as Utility>::snapshot(&request, &evaluation).is_none());
    }

    #[test]
    fn capture_lines_distinguish_absent_empty_and_matched_groups() {
        let absent = CaptureInfo {
            index: 2,
            name: None,
            value: None,
            range: None,
        };
        assert!(capture_line(&absent).contains("(unmatched)"));

        let empty = CaptureInfo {
            index: 1,
            name: Some("word".to_owned()),
            value: Some(String::new()),
            range: Some(sofdevtool_core::utilities::regex::TextRange { start: 3, end: 3 }),
        };
        assert!(capture_line(&empty).contains("word ($1) = (empty)"));

        let matched = CaptureInfo {
            index: 0,
            name: None,
            value: Some("a\nb".to_owned()),
            range: Some(sofdevtool_core::utilities::regex::TextRange { start: 0, end: 3 }),
        };
        assert_eq!(capture_line(&matched), "  $0 = a\\nb [0..3)");
    }
}
