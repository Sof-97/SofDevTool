//! The Random String workspace: configure an alphabet and batch, generate with
//! system cryptographic randomness on an explicit action, copy individual or
//! all results, and restore exact values from fresh Rust History.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, AnyElement, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window,
};
use serde::{Deserialize, Serialize};
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
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::preferences::{RandomStringControlsPreferences, RandomStringControlsStartup};
use crate::workbench::Workbench;

type RandomStringSession = Session<RandomString>;

/// The Random String Utility's persisted control configuration.
///
/// Ordinary preferences carry controls only: generated values and generation
/// bookkeeping (the request nonce) are Utility Operation data and never
/// appear here. Loading a stored configuration never generates output or
/// records History.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomStringControls {
    pub length: usize,
    pub count: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
    pub custom_alphabet: String,
}

impl RandomStringControls {
    /// The controls captured from one configuration, dropping the nonce.
    pub fn from_configuration(request: &RandomStringRequest) -> Self {
        Self {
            length: request.length,
            count: request.count,
            uppercase: request.uppercase,
            lowercase: request.lowercase,
            digits: request.digits,
            symbols: request.symbols,
            exclude_ambiguous: request.exclude_ambiguous,
            custom_alphabet: request.custom_alphabet.clone(),
        }
    }

    /// The configuration for these controls, with a fresh nonce. Restoring
    /// controls never schedules or records a generation by itself.
    pub fn to_configuration(&self) -> RandomStringRequest {
        RandomStringRequest {
            length: self.length,
            count: self.count,
            uppercase: self.uppercase,
            lowercase: self.lowercase,
            digits: self.digits,
            symbols: self.symbols,
            exclude_ambiguous: self.exclude_ambiguous,
            custom_alphabet: self.custom_alphabet.clone(),
            nonce: 0,
        }
    }

    /// Whether the controls are inside the supported length/count ranges.
    pub fn is_supported(&self) -> bool {
        self.to_configuration().has_supported_ranges()
    }
}

impl Default for RandomStringControls {
    fn default() -> Self {
        Self::from_configuration(&RandomStringRequest::default())
    }
}

/// Coordinates the Random String workspace's saved controls.
///
/// GPUI-independent so reconstruction under an isolated data root is tested
/// without a window: the workspace applies `controls()` once at construction
/// and reports every actual control change through `save_change`. Opening
/// performs no writes, generation or History recording.
pub struct RandomStringControlsSession {
    preferences: Option<RandomStringControlsPreferences>,
    controls: RandomStringControls,
    status: Option<String>,
}

impl RandomStringControlsSession {
    /// Opens the session with the latest saved controls, or the defaults when
    /// nothing valid is stored. A missing file is neutral; malformed,
    /// version-invalid or out-of-range data yields the defaults plus a
    /// diagnostic. An unavailable store still allows in-memory changes.
    pub fn open(preferences: Option<RandomStringControlsPreferences>) -> Self {
        match preferences {
            Some(preferences) => {
                let RandomStringControlsStartup {
                    controls,
                    diagnostic,
                } = preferences.load_for_startup();
                Self {
                    preferences: Some(preferences),
                    controls,
                    status: diagnostic,
                }
            }
            None => Self {
                preferences: None,
                controls: RandomStringControls::default(),
                status: Some(
                    "Random String controls will not survive relaunch because the Rust Application Support directory could not be resolved."
                        .to_owned(),
                ),
            },
        }
    }

    /// The controls a freshly constructed workspace applies.
    pub fn controls(&self) -> &RandomStringControls {
        &self.controls
    }

    /// The current load/save diagnostic, if any.
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    /// Records one actual control change. A failed save keeps the visible
    /// configuration and reports the failure honestly; a later successful
    /// save clears that status. Unchanged controls write nothing.
    pub fn save_change(&mut self, controls: RandomStringControls) {
        if controls == self.controls {
            return;
        }
        self.controls = controls;
        let Some(preferences) = &self.preferences else {
            return;
        };
        match preferences.save(&self.controls) {
            Ok(()) => self.status = None,
            Err(error) => {
                self.status = Some(format!(
                    "Random String controls could not be saved; the current configuration applies to this launch only. {error}"
                ));
            }
        }
    }
}

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
    controls: RandomStringControlsSession,
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
    history_view: HistoryViewState,
    history_visible: bool,
    _history_subscription: HistorySubscription,
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
        let controls = RandomStringControlsSession::open(
            RandomStringControlsPreferences::application_support().ok(),
        );
        Self::new_with_controls(window, cx, clipboard, history, controls)
    }

    fn new_with_controls(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
        controls: RandomStringControlsSession,
    ) -> Self {
        let custom = TextField::new(window, cx);
        // Silent assignment: loading the saved controls never emits a user
        // edit, so it cannot trigger a save, a generation or a History
        // operation. The workspace opens neutral until Generate.
        custom.set_text(controls.controls().custom_alphabet.clone(), window, cx);
        let initial = controls.controls().clone();
        let subscriptions = vec![custom.on_change_in(window, cx, |this, _window, cx| {
            this.custom_alphabet_changed(cx);
        })];
        let history_view = HistoryViewState::load(&history, RandomString::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(RandomString::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        Self {
            custom,
            clipboard,
            history,
            controls,
            length: initial.length,
            count: initial.count,
            uppercase: initial.uppercase,
            lowercase: initial.lowercase,
            digits: initial.digits,
            symbols: initial.symbols,
            exclude_ambiguous: initial.exclude_ambiguous,
            nonce: 0,
            session: RandomStringSession::new(),
            copied: None,
            history_view,
            history_visible: true,
            _history_subscription: history_subscription,
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
        let result = self
            .history
            .record(RandomString::ID, RandomString::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, RandomString::ID, result);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, RandomString::ID);
        cx.notify();
    }

    /// The controls as currently shown, read from the workspace state.
    fn current_controls(&self, cx: &App) -> RandomStringControls {
        RandomStringControls {
            length: self.length,
            count: self.count,
            uppercase: self.uppercase,
            lowercase: self.lowercase,
            digits: self.digits,
            symbols: self.symbols,
            exclude_ambiguous: self.exclude_ambiguous,
            custom_alphabet: self.custom.text(cx),
        }
    }

    /// Saves the latest controls after an actual control change,
    /// independently of generation and History recording.
    fn persist_controls(&mut self, cx: &App) {
        let controls = self.current_controls(cx);
        self.controls.save_change(controls);
    }

    fn custom_alphabet_changed(&mut self, cx: &mut Context<Self>) {
        // Editing the custom alphabet updates the preview only; generation
        // stays explicit on the Generate action.
        self.copied = None;
        self.persist_controls(cx);
        cx.notify();
    }

    fn set_length(&mut self, delta: isize, cx: &App) {
        let next = self.length as isize + delta;
        let next = next.clamp(MIN_LENGTH as isize, MAX_LENGTH as isize) as usize;
        if next != self.length {
            self.length = next;
            self.persist_controls(cx);
        }
        self.copied = None;
    }

    fn set_count(&mut self, delta: isize, cx: &App) {
        let next = self.count as isize + delta;
        let next = next.clamp(MIN_COUNT as isize, MAX_COUNT as isize) as usize;
        if next != self.count {
            self.count = next;
            self.persist_controls(cx);
        }
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
        self.history_view.pending_restore = None;
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
        let current = self.session.evaluation().values();
        let differs = !current.is_empty() && current != snapshot.values.as_slice();
        if differs {
            self.history_view.pending_restore = Some(entry);
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
        // The restored controls are now the latest configuration; saving them
        // is not a History operation and does not generate output.
        self.persist_controls(cx);
        self.session.restore(snapshot);
        self.history_view.pending_restore = None;
        self.copied = None;
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.history_view.pending_restore.clone() {
            if self
                .history_view
                .retained(&self.history, RandomString::ID, &entry)
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
                .retained(&self.history, RandomString::ID, &entry)
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
                this.persist_controls(cx);
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
        let tokens = ThemeTokens::active();
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
        let tokens = ThemeTokens::active();
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
                                this.set_length(-1, cx);
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
                                this.set_length(1, cx);
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
                                this.set_count(-1, cx);
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
                                this.set_count(1, cx);
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

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("Random String History: {error}"),
                None,
            ));
        }
        if let Some(status) = self.controls.status().map(str::to_owned) {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &status,
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use sofdevtool_core::utilities::random_string::RandomStringEvaluation;

    use super::*;
    use crate::history::{HistoryPolicy, HistoryRecorder, HistoryStore, SystemClock};

    struct TestClipboard;

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            None
        }

        fn write_text(&self, _text: &str, _cx: &mut App) {}
    }

    /// The store's file name, mirrored from `preferences.rs` for fixture
    /// setup; the store owns the authoritative constant.
    const CONTROLS_FILE: &str = "random-string-controls.v1.json";

    fn temporary_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-rs-controls-test-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ))
    }

    /// Opens a fresh controls session against the root, as a relaunch would.
    fn reopen(root: &Path) -> RandomStringControlsSession {
        RandomStringControlsSession::open(Some(RandomStringControlsPreferences::new(
            root.to_path_buf(),
        )))
    }

    fn apply_and_reopen(
        session: &mut RandomStringControlsSession,
        root: &Path,
        change: impl Fn(&mut RandomStringControls),
    ) -> RandomStringControls {
        let mut controls = session.controls().clone();
        change(&mut controls);
        session.save_change(controls);
        assert_eq!(session.status(), None);
        reopen(root).controls().clone()
    }

    #[test]
    fn default_controls_match_the_documented_product_defaults() {
        let controls = RandomStringControls::default();
        assert_eq!(controls.length, 20);
        assert_eq!(controls.count, 1);
        assert!(controls.uppercase);
        assert!(controls.lowercase);
        assert!(controls.digits);
        assert!(controls.symbols);
        assert!(controls.exclude_ambiguous);
        assert!(controls.custom_alphabet.is_empty());
        assert_eq!(
            controls.to_configuration(),
            RandomStringRequest::default().configuration()
        );
    }

    #[test]
    fn ordinary_preferences_carry_controls_only_never_values_or_bookkeeping() {
        let request = RandomStringRequest {
            nonce: 9,
            custom_alphabet: "abc".to_owned(),
            ..RandomStringRequest::default()
        };
        let controls = RandomStringControls::from_configuration(&request);

        let serialized = serde_json::to_value(&controls).expect("controls serialize");
        let object = serialized.as_object().expect("controls object");
        assert_eq!(object.len(), 8, "only the eight control fields persist");
        for excluded in ["nonce", "values", "entropy_bits", "captured_at", "id"] {
            assert!(
                !object.contains_key(excluded),
                "operation data {excluded:?} must not enter ordinary preferences"
            );
        }
        assert_eq!(controls.to_configuration().nonce, 0);
    }

    #[test]
    fn a_fresh_launch_opens_neutral_and_writes_nothing() {
        let root = temporary_root("fresh");
        let session = reopen(&root);

        assert_eq!(session.controls(), &RandomStringControls::default());
        assert_eq!(session.status(), None);
        assert!(
            !root.join(CONTROLS_FILE).exists(),
            "loading controls must not write the preference file"
        );

        // The configuration loads into a neutral Utility: no generation,
        // nothing to snapshot, nothing recordable.
        let mut generation = RandomStringSession::new();
        assert!(matches!(
            generation.evaluation(),
            RandomStringEvaluation::Empty
        ));
        assert!(generation.take_snapshot().is_none());
    }

    #[test]
    fn saved_controls_survive_relaunch_with_history_disabled_and_record_nothing() {
        let root = temporary_root("relaunch");
        let history = HistoryRecorder::with_policy(
            HistoryStore::new(root.join("History")),
            Box::new(SystemClock::new()),
            HistoryPolicy {
                global_enabled: false,
                ..HistoryPolicy::default()
            },
            None,
        );

        let changed = RandomStringControls {
            length: 48,
            count: 7,
            uppercase: false,
            lowercase: true,
            digits: false,
            symbols: false,
            exclude_ambiguous: false,
            custom_alphabet: "αβγ".to_owned(),
        };
        let mut first_launch = reopen(&root);
        first_launch.save_change(changed.clone());
        assert_eq!(first_launch.status(), None);
        drop(first_launch);

        let relaunched = reopen(&root);
        assert_eq!(relaunched.controls(), &changed);
        assert_eq!(relaunched.status(), None);

        // Saving and loading controls never recorded a Utility Operation,
        // even though the recorder shares the data root.
        assert!(history
            .load(RandomString::ID)
            .expect("load history")
            .is_empty());
        assert!(!root.join("History").exists());
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn every_supported_control_change_persists_immediately() {
        let root = temporary_root("changes");
        let mut session = reopen(&root);

        let recovered = apply_and_reopen(&mut session, &root, |controls| controls.length = 4096);
        assert_eq!(recovered.length, 4096);
        let recovered = apply_and_reopen(&mut session, &root, |controls| controls.count = 100);
        assert_eq!(recovered.count, 100);
        let recovered =
            apply_and_reopen(&mut session, &root, |controls| controls.uppercase = false);
        assert!(!recovered.uppercase);
        let recovered =
            apply_and_reopen(&mut session, &root, |controls| controls.lowercase = false);
        assert!(!recovered.lowercase);
        let recovered = apply_and_reopen(&mut session, &root, |controls| controls.digits = false);
        assert!(!recovered.digits);
        let recovered = apply_and_reopen(&mut session, &root, |controls| controls.symbols = false);
        assert!(!recovered.symbols);
        let recovered = apply_and_reopen(&mut session, &root, |controls| {
            controls.exclude_ambiguous = false
        });
        assert!(!recovered.exclude_ambiguous);
        let recovered = apply_and_reopen(&mut session, &root, |controls| {
            controls.custom_alphabet = "€λ👍".to_owned()
        });
        assert_eq!(recovered.custom_alphabet, "€λ👍");

        assert_eq!(
            recovered,
            RandomStringControls {
                length: 4096,
                count: 100,
                uppercase: false,
                lowercase: false,
                digits: false,
                symbols: false,
                exclude_ambiguous: false,
                custom_alphabet: "€λ👍".to_owned(),
            }
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn unchanged_controls_write_nothing() {
        let root = temporary_root("unchanged");
        let mut session = reopen(&root);

        session.save_change(session.controls().clone());

        assert_eq!(session.status(), None);
        assert!(!root.join(CONTROLS_FILE).exists());
    }

    #[test]
    fn a_failed_save_is_reported_and_the_next_successful_save_clears_the_status() {
        let root = temporary_root("readonly");
        fs::create_dir_all(&root).expect("create test directory");
        let mut session = reopen(&root);
        let attempted = RandomStringControls {
            length: 33,
            ..RandomStringControls::default()
        };

        // A read-only directory makes the atomic write fail deterministically.
        let mut permissions = fs::metadata(&root).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&root, permissions).unwrap();

        session.save_change(attempted.clone());

        let mut permissions = fs::metadata(&root).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&root, permissions).unwrap();

        let status = session.status().expect("a failed save is reported");
        assert!(status.contains("could not be saved"), "status: {status}");
        // The visible configuration is preserved, not reverted.
        assert_eq!(session.controls(), &attempted);
        assert!(
            !root.join(CONTROLS_FILE).exists(),
            "a failed save must not be presented as persisted"
        );

        // The next actual change retries; success clears the status.
        let recovered = apply_and_reopen(&mut session, &root, |controls| controls.count = 2);
        assert_eq!(recovered.count, 2);
        assert_eq!(recovered.length, 33);
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn malformed_controls_fall_back_to_defaults_with_a_diagnostic_and_stay_untouched() {
        let root = temporary_root("malformed");
        fs::create_dir_all(&root).expect("create test directory");
        fs::write(root.join(CONTROLS_FILE), "not json").expect("write malformed data");

        let session = reopen(&root);

        assert_eq!(session.controls(), &RandomStringControls::default());
        let status = session.status().expect("malformed data is reported");
        assert!(status.contains("could not be loaded"), "status: {status}");
        assert_eq!(
            fs::read_to_string(root.join(CONTROLS_FILE)).expect("malformed file remains"),
            "not json",
            "unusable data is never silently overwritten"
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn an_unsupported_schema_version_falls_back_to_defaults_with_a_diagnostic() {
        let root = temporary_root("version");
        fs::create_dir_all(&root).expect("create test directory");
        let record = serde_json::json!({
            "version": 99,
            "controls": serde_json::to_value(RandomStringControls::default())
                .expect("controls serialize"),
        });
        fs::write(root.join(CONTROLS_FILE), record.to_string()).expect("write record");

        let session = reopen(&root);

        assert_eq!(session.controls(), &RandomStringControls::default());
        let status = session.status().expect("the version mismatch is reported");
        assert!(
            status.contains("unsupported schema version"),
            "status: {status}"
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn out_of_range_stored_controls_are_rejected_entirely() {
        let root = temporary_root("ranges");
        fs::create_dir_all(&root).expect("create test directory");
        let path = root.join(CONTROLS_FILE);

        for (field, value) in [
            ("length", 0),
            ("length", 4097),
            ("count", 0),
            ("count", 101),
        ] {
            let mut controls =
                serde_json::to_value(RandomStringControls::default()).expect("controls");
            controls[field] = serde_json::json!(value);
            let record = serde_json::json!({ "version": 1, "controls": controls });
            fs::write(&path, record.to_string()).expect("write record");

            let session = reopen(&root);

            assert_eq!(
                session.controls(),
                &RandomStringControls::default(),
                "{field}={value} must not partially apply"
            );
            let status = session.status().expect("out-of-range data is reported");
            assert!(
                status.contains("outside the supported ranges"),
                "{field}={value} status: {status}"
            );
        }
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn controls_are_scoped_to_their_profile_data_root() {
        let root_a = temporary_root("profile-a");
        let root_b = temporary_root("profile-b");

        let mut session_a = reopen(&root_a);
        session_a.save_change(RandomStringControls {
            length: 99,
            ..RandomStringControls::default()
        });

        // Another profile root (the seam Debug/Release isolation will use)
        // resolves independently to its own state.
        assert_eq!(reopen(&root_b).controls(), &RandomStringControls::default());
        assert_eq!(reopen(&root_a).controls().length, 99);
        fs::remove_dir_all(root_a).expect("remove test directory");
        // root_b was never written: opening a session creates nothing.
        let _ = fs::remove_dir_all(root_b);
    }

    #[gpui::test]
    fn workspace_clear_preserves_generated_batch_and_restore_does_not_record(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(sofdevtool_ui::init);
        let root = temporary_root("history-workspace");
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.join("History")),
            Box::new(SystemClock::new()),
        ));
        let controls = reopen(&root);
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard);
        let (workspace, cx) = cx.add_window_view(|window, cx| {
            RandomStringWorkspace::new_with_controls(
                window,
                cx,
                clipboard,
                history.clone(),
                controls,
            )
        });
        let generated = cx.update(|_, cx| {
            workspace.update(cx, |view, cx| {
                view.generate(cx);
                view.session.evaluation().values().to_vec()
            })
        });
        assert_eq!(generated.len(), 1);
        let retained = history.load(RandomString::ID).unwrap();
        assert_eq!(retained.len(), 1);
        let entry = retained[0].clone();
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                let mut other = decode_snapshot(&entry).unwrap();
                other.values = vec!["visible-current-value".into()];
                view.session.restore(other);
                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.history_view.pending_restore, Some(entry.clone()));
                view.confirm_restore(window, cx);
                assert_eq!(view.session.evaluation().values(), generated);
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(history.load(RandomString::ID).unwrap().len(), 1);

                let mut current = decode_snapshot(&entry).unwrap();
                current.values = vec!["still-visible-after-clear".into()];
                view.session.restore(current);
                assert!(view.history_view.select(&entry.id));
                view.history_view.pending_restore = Some(entry.clone());
            });
            history.clear_utility(RandomString::ID, cx).unwrap();
            let view = workspace.read(cx);
            assert_eq!(
                view.session.evaluation().values(),
                &["still-visible-after-clear".to_owned()]
            );
            assert!(view.history_view.entries.is_empty());
            assert!(view.history_view.selected.is_none());
            assert!(view.history_view.pending_restore.is_none());

            workspace.update(cx, |view, cx| {
                view.history_view.pending_restore = Some(entry.clone());
                view.confirm_restore(window, cx);
                assert_eq!(
                    view.session.evaluation().values(),
                    &["still-visible-after-clear".to_owned()]
                );
                view.generate(cx);
                assert_eq!(view.session.evaluation().values().len(), 1);
            });
            assert_eq!(history.load(RandomString::ID).unwrap().len(), 1);
        });
        fs::remove_dir_all(root).unwrap();
    }
}
