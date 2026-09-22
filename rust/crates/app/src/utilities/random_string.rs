//! The Random String workspace: configure an alphabet and batch, generate with
//! system cryptographic randomness on an explicit action, copy individual or
//! all results, and restore exact values from fresh Rust History.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, AnyElement, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::random_string::{
    alphabet as build_alphabet, entropy_bits, RandomString, RandomStringRequest,
    RandomStringSnapshot, MAX_COUNT, MAX_LENGTH, MIN_COUNT, MIN_LENGTH,
};
use sofdevtool_core::utility::Utility;
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    DiagnosticSeverity, HistoryItem, HistoryPanel, TextField, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder};
use crate::workbench::Workbench;

type RandomStringSession = Session<RandomString>;

/// Builds the Random String workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| RandomStringWorkspace::new(window, cx, clipboard, history))
        .into()
}

/// One distinct focus handle per simultaneously-rendered button. Reusing a
/// handle across two visible buttons aborts GPUI when both request focus in a
/// single frame; per-result Copy buttons get handles from `item_focus`.
struct ButtonFocus {
    length_down: FocusHandle,
    length_up: FocusHandle,
    count_down: FocusHandle,
    count_up: FocusHandle,
    uppercase: FocusHandle,
    lowercase: FocusHandle,
    digits: FocusHandle,
    symbols: FocusHandle,
    ambiguous: FocusHandle,
    generate: FocusHandle,
    paste: FocusHandle,
    copy_all: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct RandomStringWorkspace {
    custom: TextField,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    length: usize,
    count: usize,
    uppercase: bool,
    lowercase: bool,
    digits: bool,
    symbols: bool,
    exclude_ambiguous: bool,
    nonce: u64,
    session: RandomStringSession,
    copied: Option<String>,
    history_entries: Vec<HistoryEntry>,
    history_selected: Option<String>,
    history_visible: bool,
    history_error: Option<String>,
    pending_restore: Option<HistoryEntry>,
    item_focus: Vec<FocusHandle>,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl RandomStringWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let custom = TextField::new(window, cx);
        let subscriptions = vec![custom.on_change_in(window, cx, |_this, _window, cx| {
            // Editing the custom alphabet updates the preview only; generation
            // stays explicit on the Generate action.
            cx.notify();
        })];
        let (history_entries, history_error) = match history.load(RandomString::ID) {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        Self {
            custom,
            clipboard,
            history,
            length: 20,
            count: 1,
            uppercase: true,
            lowercase: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: true,
            nonce: 0,
            session: RandomStringSession::new(),
            copied: None,
            history_entries,
            history_selected: None,
            history_visible: true,
            history_error,
            pending_restore: None,
            item_focus: Vec::new(),
            focus: ButtonFocus {
                length_down: cx.focus_handle().tab_stop(true).tab_index(0),
                length_up: cx.focus_handle().tab_stop(true).tab_index(0),
                count_down: cx.focus_handle().tab_stop(true).tab_index(0),
                count_up: cx.focus_handle().tab_stop(true).tab_index(0),
                uppercase: cx.focus_handle().tab_stop(true).tab_index(0),
                lowercase: cx.focus_handle().tab_stop(true).tab_index(0),
                digits: cx.focus_handle().tab_stop(true).tab_index(0),
                symbols: cx.focus_handle().tab_stop(true).tab_index(0),
                ambiguous: cx.focus_handle().tab_stop(true).tab_index(0),
                generate: cx.focus_handle().tab_stop(true).tab_index(0),
                paste: cx.focus_handle().tab_stop(true).tab_index(0),
                copy_all: cx.focus_handle().tab_stop(true).tab_index(0),
                clear: cx.focus_handle().tab_stop(true).tab_index(0),
                history_toggle: cx.focus_handle().tab_stop(true).tab_index(0),
                history_restore: cx.focus_handle().tab_stop(true).tab_index(0),
                history_confirm: cx.focus_handle().tab_stop(true).tab_index(0),
                history_cancel: cx.focus_handle().tab_stop(true).tab_index(0),
            },
            _subscriptions: subscriptions,
        }
    }

    /// The current configuration, without advancing the generation nonce.
    fn configuration(&self, cx: &App) -> RandomStringRequest {
        RandomStringRequest {
            length: self.length,
            count: self.count,
            uppercase: self.uppercase,
            lowercase: self.lowercase,
            digits: self.digits,
            symbols: self.symbols,
            exclude_ambiguous: self.exclude_ambiguous,
            custom_alphabet: self.custom.text(cx),
            nonce: self.nonce,
        }
    }

    /// The configuration for one deliberate generation: advances the nonce so a
    /// repeat of identical controls still schedules a fresh revision.
    fn next_request(&mut self, cx: &App) -> RandomStringRequest {
        self.nonce = self.nonce.wrapping_add(1);
        self.configuration(cx)
    }

    fn generate(&mut self, cx: &mut Context<Self>) {
        let request = self.next_request(cx);
        let SubmitOutcome::Scheduled(revision) = self.session.submit(request) else {
            return;
        };
        // Generation is fast and synchronous; settle immediately with no
        // intermediate edit ever published.
        self.session.resolve(revision);
        self.copied = None;
        self.record_settled(cx);
        cx.notify();
    }

    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a Random String snapshot serializes");
        match self
            .history
            .record(RandomString::ID, RandomString::SNAPSHOT_VERSION, payload)
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

    fn set_length(&mut self, delta: isize) {
        let next = self.length as isize + delta;
        self.length = next.clamp(MIN_LENGTH as isize, MAX_LENGTH as isize) as usize;
        self.copied = None;
    }

    fn set_count(&mut self, delta: isize) {
        let next = self.count as isize + delta;
        self.count = next.clamp(MIN_COUNT as isize, MAX_COUNT as isize) as usize;
        self.copied = None;
    }

    fn paste_custom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = None;
            self.custom.replace_all(text, window, cx);
        }
    }

    fn copy_item(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(value) = self.session.evaluation().values().get(index).cloned() else {
            return;
        };
        self.clipboard.write_text(&value, cx);
        self.copied = Some(format!("Copied result {}", index + 1));
        cx.notify();
    }

    fn copy_all(&mut self, cx: &mut Context<Self>) {
        let values = self.session.evaluation().values();
        if values.is_empty() {
            return;
        }
        let joined = values.join("\n");
        self.clipboard.write_text(&joined, cx);
        self.copied = Some(format!("Copied all {} results", values.len()));
        cx.notify();
    }

    fn clear_results(&mut self, cx: &mut Context<Self>) {
        self.session.clear();
        self.copied = None;
        self.pending_restore = None;
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
        let current = self.session.evaluation().values();
        let differs = !current.is_empty() && current != snapshot.values.as_slice();
        if differs {
            self.pending_restore = Some(entry);
            cx.notify();
        } else {
            self.apply_restore(snapshot, window, cx);
        }
    }

    fn apply_restore(
        &mut self,
        snapshot: RandomStringSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let request = snapshot.request.clone();
        self.length = request.length;
        self.count = request.count;
        self.uppercase = request.uppercase;
        self.lowercase = request.lowercase;
        self.digits = request.digits;
        self.symbols = request.symbols;
        self.exclude_ambiguous = request.exclude_ambiguous;
        self.custom
            .set_text(request.custom_alphabet.clone(), window, cx);
        // Never let a later deliberate generation reuse an already-recorded
        // nonce for the same configuration.
        self.nonce = self.nonce.max(request.nonce);
        self.session.restore(snapshot);
        self.pending_restore = None;
        self.copied = None;
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
                        .map(|snapshot| preview_line(&snapshot.values))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn toggle_button(
        &self,
        label: &'static str,
        on: bool,
        handle: FocusHandle,
        cx: &mut Context<Self>,
        toggle: impl Fn(&mut Self) + 'static,
    ) -> Button {
        Button::new(label)
            .variant(if on {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(handle)
            .on_click(view_click(cx, move |this, _window, cx| {
                toggle(this);
                this.copied = None;
                cx.notify();
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

    fn render_results(&self, cx: &mut Context<Self>) -> AnyElement {
        let tokens = ThemeTokens::graphite();
        let values = self.session.evaluation().values();
        if values.is_empty() {
            return empty_state("Configure an alphabet, then choose Generate").into_any_element();
        }
        let mut list = div()
            .id("random-results")
            .role(gpui::Role::ListBox)
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .overflow_y_scroll();
        for (index, value) in values.iter().enumerate() {
            let mut copy = Button::new(format!("Copy {}", index + 1));
            if let Some(handle) = self.item_focus.get(index).cloned() {
                copy = copy.focus_handle(handle);
            }
            let row = div()
                .flex()
                .flex_row()
                .items_start()
                .gap_2()
                .w_full()
                .px_2()
                .py_2()
                .rounded_md()
                .border_1()
                .border_color(tokens.border())
                .bg(tokens.surface())
                .child(
                    div()
                        .w_6()
                        .text_xs()
                        .text_color(tokens.text_muted())
                        .child(format!("{}", index + 1)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .text_color(tokens.text())
                        .child(value.clone()),
                )
                .child(copy.on_click(view_click(cx, move |this, _window, cx| {
                    this.copy_item(index, cx);
                })));
            list = list.child(row);
        }
        list.into_any_element()
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
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text())
                    .child("Restoring this History entry replaces the current generated results."),
            )
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

impl Render for RandomStringWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
        let needed = self.session.evaluation().values().len();
        while self.item_focus.len() < needed {
            self.item_focus
                .push(cx.focus_handle().tab_stop(true).tab_index(0));
        }

        let preview = self.configuration(cx);
        let characters = build_alphabet(&preview);
        let entropy = entropy_bits(self.length, characters.len());
        let summary = match entropy {
            Some(bits) => format!(
                "Alphabet: {} characters · entropy per result: {:.1} bits",
                characters.len(),
                bits
            ),
            None => format!("Alphabet: {} characters", characters.len()),
        };
        let has_results = !self.session.evaluation().values().is_empty();

        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Random String"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("System randomness for test data · local and offline"),
            );

        let classes = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(self.toggle_button(
                "A-Z",
                self.uppercase,
                self.focus.uppercase.clone(),
                cx,
                |this| this.uppercase = !this.uppercase,
            ))
            .child(self.toggle_button(
                "a-z",
                self.lowercase,
                self.focus.lowercase.clone(),
                cx,
                |this| this.lowercase = !this.lowercase,
            ))
            .child(
                self.toggle_button("0-9", self.digits, self.focus.digits.clone(), cx, |this| {
                    this.digits = !this.digits
                }),
            )
            .child(self.toggle_button(
                "Symbols",
                self.symbols,
                self.focus.symbols.clone(),
                cx,
                |this| this.symbols = !this.symbols,
            ))
            .child(self.toggle_button(
                "Exclude ambiguous",
                self.exclude_ambiguous,
                self.focus.ambiguous.clone(),
                cx,
                |this| this.exclude_ambiguous = !this.exclude_ambiguous,
            ));

        let numbers = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
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
                            .child("Length"),
                    )
                    .child(
                        Button::new("Length -")
                            .focus_handle(self.focus.length_down.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                this.set_length(-1);
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .w_12()
                            .text_center()
                            .text_sm()
                            .child(self.length.to_string()),
                    )
                    .child(
                        Button::new("Length +")
                            .focus_handle(self.focus.length_up.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                this.set_length(1);
                                cx.notify();
                            })),
                    ),
            )
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
                            .child("Count"),
                    )
                    .child(
                        Button::new("Count -")
                            .focus_handle(self.focus.count_down.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                this.set_count(-1);
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .w_12()
                            .text_center()
                            .text_sm()
                            .child(self.count.to_string()),
                    )
                    .child(
                        Button::new("Count +")
                            .focus_handle(self.focus.count_up.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                this.set_count(1);
                                cx.notify();
                            })),
                    ),
            );

        let custom = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Custom characters"),
            )
            .child(self.custom.render("random.custom"));

        let actions = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                Button::new("Generate")
                    .variant(ButtonVariant::Primary)
                    .focus_handle(self.focus.generate.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.generate(cx);
                    })),
            )
            .child(
                Button::new("Paste custom")
                    .focus_handle(self.focus.paste.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.paste_custom(window, cx);
                    })),
            )
            .child(
                Button::new("Copy All")
                    .disabled(!has_results)
                    .focus_handle(self.focus.copy_all.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_all(cx);
                    })),
            )
            .child(
                Button::new("Clear results")
                    .disabled(!has_results)
                    .focus_handle(self.focus.clear.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.clear_results(cx);
                    })),
            )
            .child(div().flex_1())
            .child(copy_feedback(
                self.copied.is_some(),
                self.copied.as_deref().unwrap_or(""),
            ))
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
            .child(header)
            .child(classes)
            .child(numbers)
            .child(custom)
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child(summary),
            )
            .child(actions);

        if self.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("History is paused for Random String: {error}"),
                None,
            ));
        }
        column = column.child(self.render_diagnostics());

        let caption = format!(
            "{} result{}",
            self.session.evaluation().values().len(),
            if self.session.evaluation().values().len() == 1 {
                ""
            } else {
                "s"
            }
        );
        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_4()
            .flex_1()
            .min_h_0()
            .child(panel("Generated Results", caption, self.render_results(cx)));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace)
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<RandomStringSnapshot> {
    if entry.snapshot_version != RandomString::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

fn preview_line(values: &[String]) -> String {
    match values {
        [] => "No results".to_owned(),
        [only] => truncate(only),
        _ => {
            let joined = values.join(", ");
            truncate(&joined)
        }
    }
}

fn truncate(value: &str) -> String {
    let single_line = value.replace('\n', " ");
    let trimmed = single_line.trim();
    let mut preview: String = trimmed.chars().take(80).collect();
    if trimmed.chars().count() > 80 {
        preview.push('…');
    }
    preview
}
