//! The Random String workspace: configure an alphabet and batch, generate with
//! system cryptographic randomness on an explicit action, copy individual or
//! all results, and restore exact values from fresh Rust History.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, uniform_list, AnyElement, App, Context, Entity, IntoElement, Render, Subscription,
    Window,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    input::{Input, InputEvent, InputState, NumberInput},
    list::ListState,
    ActiveTheme as _, Disableable as _, Icon,
};
use serde::{Deserialize, Serialize};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::random_string::{
    alphabet as build_alphabet, entropy_bits, RandomString, RandomStringRequest,
    RandomStringSnapshot, MAX_COUNT, MAX_LENGTH, MIN_COUNT, MIN_LENGTH,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{
    HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState, RETENTION,
};
use crate::preferences::{RandomStringControlsPreferences, RandomStringControlsStartup};
use crate::registry::{UtilityId, WorkspaceViews};
use crate::ui;
use crate::workbench::Workbench;
use crate::workspace_layout::{HistoryPlacement, WorkspaceLayout};

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
    layout: Entity<WorkspaceLayout>,
) -> WorkspaceViews {
    let workspace =
        cx.new(|cx| RandomStringWorkspace::new_with_layout(window, cx, clipboard, history, layout));
    let inspector = cx.new(|cx| {
        ui::HistoryInspector::new(
            workspace.clone(),
            |workspace, cx| workspace.render_history(cx).into_any_element(),
            cx,
        )
    });
    WorkspaceViews {
        body: workspace.into(),
        history: inspector.into(),
    }
}

pub struct RandomStringWorkspace {
    custom: Entity<InputState>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    controls: RandomStringControlsSession,
    length: usize,
    count: usize,
    length_input: Entity<InputState>,
    count_input: Entity<InputState>,
    uppercase: bool,
    lowercase: bool,
    digits: bool,
    symbols: bool,
    exclude_ambiguous: bool,
    nonce: u64,
    session: RandomStringSession,
    copied: Option<String>,
    history_view: HistoryViewState,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    layout: Entity<WorkspaceLayout>,
    _layout_subscription: Subscription,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl RandomStringWorkspace {
    fn new_with_layout(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
        layout: Entity<WorkspaceLayout>,
    ) -> Self {
        let controls = RandomStringControlsSession::open(
            RandomStringControlsPreferences::application_support().ok(),
        );
        Self::new_with_controls_and_layout(window, cx, clipboard, history, controls, layout)
    }

    #[cfg(test)]
    fn new_with_controls(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
        controls: RandomStringControlsSession,
    ) -> Self {
        let layout = cx.new(|_| WorkspaceLayout::load(None));
        Self::new_with_controls_and_layout(window, cx, clipboard, history, controls, layout)
    }

    fn new_with_controls_and_layout(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
        controls: RandomStringControlsSession,
        layout: Entity<WorkspaceLayout>,
    ) -> Self {
        let layout_subscription = cx.observe(&layout, |_, _, cx| cx.notify());
        let custom = cx.new(|cx| InputState::new(window, cx));
        // Silent assignment: loading the saved controls never emits a user
        // edit, so it cannot trigger a save, a generation or a History
        // operation. The workspace opens neutral until Generate.
        custom.update(cx, |state, cx| {
            state.set_value(controls.controls().custom_alphabet.clone(), window, cx)
        });
        let initial = controls.controls().clone();
        let length_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial.length.to_string())
                .min(MIN_LENGTH as f64)
                .max(MAX_LENGTH as f64)
                .step(1_f64)
        });
        let count_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial.count.to_string())
                .min(MIN_COUNT as f64)
                .max(MAX_COUNT as f64)
                .step(1_f64)
        });
        let subscriptions = vec![
            cx.subscribe_in(
                &custom,
                window,
                |this, _entity, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.custom_alphabet_changed(cx);
                    }
                },
            ),
            cx.subscribe_in(
                &length_input,
                window,
                |this, _entity, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.commit_length(cx);
                    }
                },
            ),
            cx.subscribe_in(
                &count_input,
                window,
                |this, _entity, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.commit_count(cx);
                    }
                },
            ),
        ];
        let history_view = HistoryViewState::load(&history, RandomString::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(RandomString::ID, move |cx| {
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
                        cx.notify();
                    }
                })
                .ok();
            }),
        );
        let workspace = Self {
            custom,
            clipboard,
            history,
            controls,
            length: initial.length,
            count: initial.count,
            length_input,
            count_input,
            uppercase: initial.uppercase,
            lowercase: initial.lowercase,
            digits: initial.digits,
            symbols: initial.symbols,
            exclude_ambiguous: initial.exclude_ambiguous,
            nonce: 0,
            session: RandomStringSession::new(),
            copied: None,
            history_view,
            history_list,
            layout,
            _layout_subscription: layout_subscription,
            _history_subscription: history_subscription,
            _subscriptions: subscriptions,
        };
        workspace.sync_history(cx);
        workspace
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
            custom_alphabet: self.custom.read(cx).value().to_string(),
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
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, RandomString::ID);
        self.sync_history(cx);
        cx.notify();
    }

    /// Reflects the owning view's History rows and selection into the kit list.
    fn sync_history(&self, cx: &mut Context<Self>) {
        ui::history_set_rows(&self.history_list, self.history_items(), cx);
        ui::history_set_selected(&self.history_list, self.history_view.selected.clone(), cx);
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
            custom_alphabet: self.custom.read(cx).value().to_string(),
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

    /// Clamps the typed/number-stepped length back into range. The retained
    /// number input is reflected during render.
    fn commit_length(&mut self, cx: &mut Context<Self>) {
        let raw = self.length_input.read(cx).value().to_string();
        let Ok(parsed) = raw.trim().parse::<isize>() else {
            self.copied = None;
            cx.notify();
            return;
        };
        let next = parsed.clamp(MIN_LENGTH as isize, MAX_LENGTH as isize) as usize;
        if next != self.length {
            self.length = next;
            self.persist_controls(cx);
        }
        self.copied = None;
        cx.notify();
    }

    /// Clamps the typed/number-stepped count back into range. The retained
    /// number input is reflected during render.
    fn commit_count(&mut self, cx: &mut Context<Self>) {
        let raw = self.count_input.read(cx).value().to_string();
        let Ok(parsed) = raw.trim().parse::<isize>() else {
            self.copied = None;
            cx.notify();
            return;
        };
        let next = parsed.clamp(MIN_COUNT as isize, MAX_COUNT as isize) as usize;
        if next != self.count {
            self.count = next;
            self.persist_controls(cx);
        }
        self.copied = None;
        cx.notify();
    }

    fn paste_custom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = None;
            self.custom
                .update(cx, |state, cx| state.replace_all(text, window, cx));
        }
    }

    fn copy_item(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(value) = self.session.evaluation().values().get(index) else {
            return;
        };
        self.clipboard.write_text(value, cx);
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
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current Random String session.",
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
        self.custom.update(cx, |state, cx| {
            state.set_value(request.custom_alphabet.clone(), window, cx)
        });
        // Never let a later deliberate generation reuse an already-recorded
        // nonce for the same configuration.
        self.nonce = self.nonce.max(request.nonce);
        // The restored controls are now the latest configuration; saving them
        // is not a History operation and does not generate output.
        self.persist_controls(cx);
        self.session.restore(snapshot);
        self.history_view.pending_restore = None;
        self.copied = None;
        self.sync_history(cx);
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
                        .map(|snapshot| preview_line(&snapshot.values))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: snapshot.is_none().then(|| "Unavailable".to_owned()),
                    selectable: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn class_checkbox(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Self, bool) + 'static,
    ) -> Checkbox {
        let weak = cx.weak_entity();
        Checkbox::new(id)
            .label(label)
            .checked(checked)
            .on_change(move |checked, _, cx| {
                weak.update(cx, |this, cx| {
                    set(this, *checked);
                    this.persist_controls(cx);
                    this.copied = None;
                    cx.notify();
                })
                .ok();
            })
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

    fn render_results(&self, cx: &mut Context<Self>) -> AnyElement {
        let count = self.session.evaluation().values().len();
        if count == 0 {
            return ui::empty_state(cx, "Configure an alphabet, then choose Generate")
                .into_any_element();
        }

        let workspace = cx.entity();
        uniform_list("random-results", count, move |range, _window, cx| {
            let state = workspace.read(cx);
            let theme = cx.theme().clone();
            range
                .map(|index| {
                    let value = state.session.evaluation().values()[index].clone();
                    let workspace = workspace.clone();
                    div().h(px(52.)).w_full().px_2().py_1().child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .size_full()
                            .px_3()
                            .rounded(theme.radius)
                            .bg(theme.secondary)
                            .child(
                                div()
                                    .w_6()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{}", index + 1)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child(value),
                            )
                            .child(
                                Button::new(format!("random-string.copy.{index}"))
                                    .icon(Icon::new(IconName::Copy))
                                    .ghost()
                                    .tooltip(format!("Copy result {}", index + 1))
                                    .accessibility_label(format!("Copy result {}", index + 1))
                                    .on_click(move |_event, _window, cx| {
                                        workspace.update(cx, |this, cx| this.copy_item(index, cx));
                                    }),
                            ),
                    )
                })
                .collect::<Vec<_>>()
        })
        .flex_1()
        .min_h_0()
        .w_full()
        .into_any_element()
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
            .map(|entry| decode_snapshot(entry).is_some())
            .unwrap_or(false);
        let restore = Button::new("random-string.history.restore-selected")
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
                format!("{}/{}", self.history_view.entries.len(), RETENTION),
                restore,
            ))
    }
}

impl Render for RandomStringWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        // Keep the retained number inputs showing the clamped controls.
        // Programmatic `set_value` emits no change, so this cannot loop.
        let length_text = self.length.to_string();
        self.length_input.update(cx, |state, cx| {
            if state.value().as_ref() != length_text.as_str() {
                state.set_value(length_text.clone(), window, cx);
            }
        });
        let count_text = self.count.to_string();
        self.count_input.update(cx, |state, cx| {
            if state.value().as_ref() != count_text.as_str() {
                state.set_value(count_text.clone(), window, cx);
            }
        });

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

        let classes = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .flex_wrap()
            .child(self.class_checkbox(
                "random-string.class.uppercase",
                "A-Z",
                self.uppercase,
                cx,
                |this, checked| this.uppercase = checked,
            ))
            .child(self.class_checkbox(
                "random-string.class.lowercase",
                "a-z",
                self.lowercase,
                cx,
                |this, checked| this.lowercase = checked,
            ))
            .child(self.class_checkbox(
                "random-string.class.digits",
                "0-9",
                self.digits,
                cx,
                |this, checked| this.digits = checked,
            ))
            .child(self.class_checkbox(
                "random-string.class.symbols",
                "Symbols",
                self.symbols,
                cx,
                |this, checked| this.symbols = checked,
            ))
            .child(self.class_checkbox(
                "random-string.class.exclude-ambiguous",
                "Exclude ambiguous",
                self.exclude_ambiguous,
                cx,
                |this, checked| this.exclude_ambiguous = checked,
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
                            .text_color(theme.muted_foreground)
                            .child("Length"),
                    )
                    .child(div().w_32().child(NumberInput::new(&self.length_input))),
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
                            .text_color(theme.muted_foreground)
                            .child("Count"),
                    )
                    .child(div().w_32().child(NumberInput::new(&self.count_input))),
            );

        let custom = div()
            .flex()
            .flex_col()
            .min_w_0()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Custom characters")
                    .child(
                        Button::new("random-string.paste-custom")
                            .icon(Icon::new(IconName::ClipboardPaste))
                            .tooltip("Paste custom characters")
                            .accessibility_label("Paste custom characters")
                            .on_click(cx.listener(|this, _event, window, cx| {
                                this.paste_custom(window, cx);
                            })),
                    ),
            )
            .child(
                Input::new(&self.custom)
                    .accessibility_id("random.custom")
                    .w_full(),
            );

        let actions = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                Button::new("random-string.generate")
                    .label("Generate")
                    .primary()
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.generate(cx);
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(summary),
            );

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(theme.background)
            .text_color(theme.foreground)
            .p_3()
            .gap_2()
            .child(classes)
            .child(numbers)
            .child(custom)
            .child(actions);

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Random String History: {error}"),
                None,
            ));
        }
        if let Some(status) = self.controls.status().map(str::to_owned) {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &status,
                None,
            ));
        }
        column = column.child(self.render_diagnostics(cx));

        let caption = format!(
            "{} result{}",
            self.session.evaluation().values().len(),
            if self.session.evaluation().values().len() == 1 {
                ""
            } else {
                "s"
            }
        );
        let results_body = self.render_results(cx);
        let result_actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(caption),
            )
            .child(
                Button::new("random-string.copy-all")
                    .icon(Icon::new(IconName::Copy))
                    .tooltip("Copy all results")
                    .accessibility_label("Copy all results")
                    .disabled(!has_results)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.copy_all(cx);
                    })),
            )
            .child(
                Button::new("random-string.clear-results")
                    .icon(Icon::new(IconName::Trash))
                    .tooltip("Clear results")
                    .accessibility_label("Clear results")
                    .disabled(!has_results)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.clear_results(cx);
                    })),
            )
            .child(ui::copy_feedback(
                cx,
                self.copied.is_some(),
                self.copied.as_deref().unwrap_or(""),
            ));
        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_4()
            .flex_1()
            .min_h_0()
            .child(ui::pane(
                cx,
                "Generated Results",
                result_actions,
                results_body,
            ));
        if self.layout.read(cx).placement(UtilityId::RandomString) == HistoryPlacement::Inline {
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
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};

    use sofdevtool_core::utilities::random_string::RandomStringEvaluation;

    use gpui::{Focusable as _, VisualTestContext};
    use gpui_kit::component::Root;

    use super::*;
    use crate::history::{HistoryPolicy, HistoryRecorder, HistoryStore, SystemClock};

    #[derive(Default)]
    struct TestClipboard(RefCell<Option<String>>);

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            None
        }

        fn write_text(&self, text: &str, _cx: &mut App) {
            *self.0.borrow_mut() = Some(text.to_owned());
        }
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
        cx.update(gpui_kit::init);
        let root = temporary_root("history-workspace");
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.join("History")),
            Box::new(SystemClock::new()),
        ));
        let controls = reopen(&root);
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| {
                RandomStringWorkspace::new_with_controls(
                    window,
                    cx,
                    clipboard,
                    history.clone(),
                    controls,
                )
            });
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

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

    #[gpui::test]
    fn repeated_generation_and_copy_actions_preserve_exact_batch_values(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let root = temporary_root("copy-actions");
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.join("History")),
            Box::new(SystemClock::new()),
        ));
        let clipboard = Rc::new(TestClipboard::default());
        let app_clipboard: Rc<dyn Clipboard> = clipboard.clone();
        let controls = reopen(&root);
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| {
                RandomStringWorkspace::new_with_controls(
                    window,
                    cx,
                    app_clipboard,
                    history.clone(),
                    controls,
                )
            });
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        // The kit number input owns stepping: ArrowUp reaches the workspace
        // through the same Change path as the stepper button.
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).count_input.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("up");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(workspace.read_with(&cx, |view, _| view.count), 2);

        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| {
                view.generate(cx);
                view.generate(cx);
            });
        });
        let values =
            workspace.read_with(&cx, |view, _| view.session.evaluation().values().to_vec());
        assert_eq!(values.len(), 2);
        assert_eq!(history.load(RandomString::ID).unwrap().len(), 2);

        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_item(1, cx));
        });
        assert_eq!(clipboard.0.borrow().as_deref(), Some(values[1].as_str()));

        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_all(cx));
        });
        assert_eq!(
            clipboard.0.borrow().as_deref(),
            Some(values.join("\n").as_str())
        );
        assert_eq!(history.load(RandomString::ID).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
