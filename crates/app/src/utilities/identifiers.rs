//! The Identifier Generator workspace: explicit UUID generation and inspection
//! with version-specific controls, per-item Clipboard actions and fresh Rust
//! History. Generation is never implicit: every Generate or Validate is a
//! deliberate action that records its own settled operation.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, uniform_list, AnyElement, App, Context, Entity, IntoElement, Render, Subscription,
    Window,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState, NumberInput},
    list::ListState,
    tab::{Tab, TabBar},
    ActiveTheme as _, Disableable as _, Icon,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::identifiers::{
    decode_ulid, IdentifierAction, IdentifierFormat, Identifiers, IdentifiersRequest,
    IdentifiersSnapshot, UlidMode, UuidVersion, DEFAULT_NAMESPACE, MAXIMUM_GENERATED_COUNT,
    MAXIMUM_ORDERED_KSUID_COUNT,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{
    HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState, RETENTION,
};
use crate::registry::{UtilityId, WorkspaceViews};
use crate::ui;
use crate::workbench::Workbench;
use crate::workspace_layout::{HistoryPlacement, WorkspaceLayout};

type IdentifiersSession = Session<Identifiers>;

/// Builds the Identifier Generator workspace as a type-erased view for the
/// Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    layout: Entity<WorkspaceLayout>,
) -> WorkspaceViews {
    let workspace =
        cx.new(|cx| IdentifiersWorkspace::new_with_layout(window, cx, clipboard, history, layout));
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

pub struct IdentifiersWorkspace {
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    format: IdentifierFormat,
    version: UuidVersion,
    count: u32,
    count_input: Entity<InputState>,
    uppercase: bool,
    hyphenated: bool,
    ulid_mode: UlidMode,
    ordered_ksuid: bool,
    previous_ulid: Option<[u8; 16]>,
    namespace: Entity<InputState>,
    name: Entity<InputState>,
    inspect: Entity<InputState>,
    session: IdentifiersSession,
    generation: u64,
    copied_all: bool,
    copied_index: Option<usize>,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    layout: Entity<WorkspaceLayout>,
    _layout_subscription: Subscription,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl IdentifiersWorkspace {
    #[cfg(test)]
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let layout = cx.new(|_| WorkspaceLayout::load(None));
        Self::new_with_layout(window, cx, clipboard, history, layout)
    }

    fn new_with_layout(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
        layout: Entity<WorkspaceLayout>,
    ) -> Self {
        let layout_subscription = cx.observe(&layout, |_, _, cx| cx.notify());
        let namespace = cx.new(|cx| InputState::new(window, cx));
        let name = cx.new(|cx| InputState::new(window, cx));
        let inspect = cx.new(|cx| InputState::new(window, cx));
        namespace.update(cx, |state, cx| {
            state.set_value(DEFAULT_NAMESPACE, window, cx)
        });
        let count_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("1")
                .min(1.0)
                .max(f64::from(MAXIMUM_ORDERED_KSUID_COUNT))
                .step(1_f64)
        });
        let subscriptions = vec![
            cx.subscribe_in(
                &namespace,
                window,
                |this, _entity, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.invalidate(cx);
                    }
                },
            ),
            cx.subscribe_in(
                &name,
                window,
                |this, _entity, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.invalidate(cx);
                    }
                },
            ),
            cx.subscribe_in(
                &inspect,
                window,
                |this, _entity, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.invalidate(cx);
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
        let history_view = HistoryViewState::load(&history, Identifiers::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Identifiers::ID, move |cx| {
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
            clipboard,
            history,
            format: IdentifierFormat::Uuid,
            version: UuidVersion::V4,
            count: 1,
            count_input,
            uppercase: false,
            hyphenated: true,
            ulid_mode: UlidMode::Random,
            ordered_ksuid: false,
            previous_ulid: None,
            namespace,
            name,
            inspect,
            session: IdentifiersSession::new(),
            generation: 0,
            copied_all: false,
            copied_index: None,
            suppress_changes: false,
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

    fn request(&self, action: IdentifierAction, cx: &App) -> IdentifiersRequest {
        IdentifiersRequest {
            action,
            format: self.format,
            version: self.version,
            count: self.count,
            uppercase: self.uppercase,
            hyphenated: self.hyphenated,
            namespace: self.namespace.read(cx).value().to_string(),
            name: self.name.read(cx).value().to_string(),
            input: self.inspect.read(cx).value().to_string(),
            ulid_mode: self.ulid_mode,
            ordered_ksuid: self.ordered_ksuid,
            previous_ulid: self.previous_ulid,
            generation: self.generation,
        }
    }

    fn signature(&self, cx: &App) -> WorkspaceSignature {
        WorkspaceSignature {
            format: self.format,
            version: self.version,
            count: self.count,
            uppercase: self.uppercase,
            hyphenated: self.hyphenated,
            ulid_mode: self.ulid_mode,
            ordered_ksuid: self.ordered_ksuid,
            namespace: self.namespace.read(cx).value().to_string(),
            name: self.name.read(cx).value().to_string(),
            input: self.inspect.read(cx).value().to_string(),
        }
    }

    /// The largest batch the active format allows. Only an explicit ordered
    /// KSUID batch may use the 16-bit sequence space.
    fn maximum_count(&self) -> u32 {
        if self.format == IdentifierFormat::Ksuid && self.ordered_ksuid {
            MAXIMUM_ORDERED_KSUID_COUNT
        } else {
            MAXIMUM_GENERATED_COUNT
        }
    }

    /// Clears stale visible output after any input or option change. The next
    /// result appears only after an explicit Generate or Validate.
    fn invalidate(&mut self, cx: &mut Context<Self>) {
        if self.suppress_changes {
            return;
        }
        self.session.clear();
        self.copied_all = false;
        self.copied_index = None;
        cx.notify();
    }

    fn run_action(&mut self, action: IdentifierAction, cx: &mut Context<Self>) {
        self.generation = self.generation.wrapping_add(1);
        let request = self.request(action, cx);
        let SubmitOutcome::Scheduled(revision) = self.session.submit(request) else {
            return;
        };
        self.session.resolve(revision);
        if action == IdentifierAction::Generate
            && self.format == IdentifierFormat::Ulid
            && self.ulid_mode == UlidMode::Monotonic
        {
            // Carry the last value forward so a later deliberate batch keeps
            // process-local ordering within the same millisecond.
            self.previous_ulid = self
                .session
                .evaluation()
                .values()
                .last()
                .and_then(|value| decode_ulid(value));
        }
        self.copied_all = false;
        self.copied_index = None;
        self.record_settled(cx);
        cx.notify();
    }

    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("an Identifier snapshot serializes");
        let result = self
            .history
            .record(Identifiers::ID, Identifiers::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Identifiers::ID, result);
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Identifiers::ID);
        self.sync_history(cx);
        cx.notify();
    }

    /// Reflects the owning view's History rows and selection into the kit list.
    fn sync_history(&self, cx: &mut Context<Self>) {
        ui::history_set_rows(&self.history_list, self.history_items(), cx);
        ui::history_set_selected(&self.history_list, self.history_view.selected.clone(), cx);
    }

    fn set_format(&mut self, format: IdentifierFormat, cx: &mut Context<Self>) {
        self.format = format;
        self.count = self.count.clamp(1, self.maximum_count());
        self.invalidate(cx);
    }

    fn set_version(&mut self, version: UuidVersion, cx: &mut Context<Self>) {
        self.version = version;
        self.invalidate(cx);
    }

    fn set_ulid_mode(&mut self, mode: UlidMode, cx: &mut Context<Self>) {
        self.ulid_mode = mode;
        self.invalidate(cx);
    }

    fn toggle_ordered_ksuid(&mut self, cx: &mut Context<Self>) {
        self.ordered_ksuid = !self.ordered_ksuid;
        self.count = self.count.clamp(1, self.maximum_count());
        self.invalidate(cx);
    }

    fn toggle_uppercase(&mut self, cx: &mut Context<Self>) {
        self.uppercase = !self.uppercase;
        self.invalidate(cx);
    }

    fn toggle_hyphens(&mut self, cx: &mut Context<Self>) {
        self.hyphenated = !self.hyphenated;
        self.invalidate(cx);
    }

    /// Clamps the typed/number-stepped count back into the active format's
    /// batch limit. The retained number input is reflected during render.
    fn commit_count(&mut self, cx: &mut Context<Self>) {
        let raw = self.count_input.read(cx).value().to_string();
        let Ok(parsed) = raw.trim().parse::<i64>() else {
            return;
        };
        let next = parsed.clamp(1, i64::from(self.maximum_count())) as u32;
        if next != self.count {
            self.count = next;
            self.invalidate(cx);
        }
    }

    fn paste_inspect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.suppress_changes = true;
            self.inspect
                .update(cx, |state, cx| state.replace_all(text, window, cx));
            self.suppress_changes = false;
            self.invalidate(cx);
        }
    }

    fn copy_value(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(value) = self.session.evaluation().values().get(index) else {
            return;
        };
        self.clipboard.write_text(value, cx);
        self.copied_all = false;
        self.copied_index = Some(index);
        cx.notify();
    }

    fn copy_all(&mut self, cx: &mut Context<Self>) {
        let values = self.session.evaluation().values();
        if values.is_empty() {
            return;
        }
        let joined = values.join("\n");
        self.clipboard.write_text(&joined, cx);
        self.copied_all = true;
        self.copied_index = None;
        cx.notify();
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.suppress_changes = true;
        self.inspect
            .update(cx, |state, cx| state.replace_all("", window, cx));
        self.suppress_changes = false;
        self.session.clear();
        self.copied_all = false;
        self.copied_index = None;
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
        let current_signature = self.signature(cx);
        let baseline_signature = WorkspaceSignature::of(&IdentifiersRequest::default());
        let nonempty = self.session.evaluation().is_valid_operation()
            || current_signature != baseline_signature;
        if restore_decision(
            nonempty,
            &current_signature,
            &WorkspaceSignature::of(&snapshot.request),
            self.session.evaluation().values(),
            snapshot.values.as_slice(),
        ) == RestoreDecision::Confirm
        {
            self.history_view.pending_restore = Some(entry);
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current non-empty Identifier Generator session.",
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
        snapshot: IdentifiersSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.format = snapshot.request.format;
        self.version = snapshot.request.version;
        self.count = snapshot.request.count;
        self.uppercase = snapshot.request.uppercase;
        self.hyphenated = snapshot.request.hyphenated;
        self.ulid_mode = snapshot.request.ulid_mode;
        self.ordered_ksuid = snapshot.request.ordered_ksuid;
        // Continue monotonic ordering from the restored final value. This is a
        // pure decode, so restore reads no clock and no randomness.
        self.previous_ulid = if snapshot.request.format == IdentifierFormat::Ulid
            && snapshot.request.ulid_mode == UlidMode::Monotonic
        {
            snapshot.values.last().and_then(|value| decode_ulid(value))
        } else {
            None
        };
        self.namespace.update(cx, |state, cx| {
            state.set_value(snapshot.request.namespace.clone(), window, cx)
        });
        self.name.update(cx, |state, cx| {
            state.set_value(snapshot.request.name.clone(), window, cx)
        });
        self.inspect.update(cx, |state, cx| {
            state.set_value(snapshot.request.input.clone(), window, cx)
        });
        self.session.restore(snapshot);
        // Advance past the restored nonce so the next explicit action is always
        // a new revision rather than being deduplicated as an unchanged request.
        self.generation = self
            .generation
            .max(self.session.request().map_or(0, |r| r.generation));
        self.suppress_changes = false;
        self.history_view.pending_restore = None;
        self.copied_all = false;
        self.copied_index = None;
        self.sync_history(cx);
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.history_view.pending_restore.clone() {
            if self
                .history_view
                .retained(&self.history, Identifiers::ID, &entry)
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
                .retained(&self.history, Identifiers::ID, &entry)
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

    fn render_values(&self, cx: &mut Context<Self>) -> AnyElement {
        let count = self.session.evaluation().values().len();
        if count == 0 {
            return ui::empty_state(cx, "Generate identifiers or validate one to begin")
                .into_any_element();
        }

        let workspace = cx.entity();
        uniform_list("identifiers.results", count, move |range, _window, cx| {
            let state = workspace.read(cx);
            let theme = cx.theme().clone();
            range
                .map(|index| {
                    let value = state.session.evaluation().values()[index].clone();
                    let copied = state.copied_index == Some(index);
                    let workspace = workspace.clone();
                    div().h(px(48.)).w_full().px_2().py_1().child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .size_full()
                            .px_3()
                            .rounded(theme.radius)
                            .bg(theme.secondary)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child(value),
                            )
                            .child(ui::copy_feedback(cx, copied, "Copied"))
                            .child(
                                Button::new(format!("identifiers.copy.{index}"))
                                    .icon(Icon::new(IconName::Copy))
                                    .tooltip(format!("Copy identifier {}", index + 1))
                                    .accessibility_label(format!("Copy identifier {}", index + 1))
                                    .ghost()
                                    .on_click(move |_event, _window, cx| {
                                        workspace.update(cx, |this, cx| this.copy_value(index, cx));
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
            .map(|entry| decode_snapshot(entry).is_some())
            .unwrap_or(false);
        let restore = Button::new("identifiers.history.restore-selected")
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

impl Render for IdentifiersWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let can_copy = !self.session.evaluation().values().is_empty();

        // Keep the retained count input showing the clamped batch size and the
        // active format's maximum. Programmatic setters emit no change, so this
        // cannot loop.
        let count_text = self.count.to_string();
        let maximum = self.maximum_count() as f64;
        self.count_input.update(cx, |state, cx| {
            state.set_max(Some(maximum), window, cx);
            if state.value().as_ref() != count_text.as_str() {
                state.set_value(count_text.clone(), window, cx);
            }
        });

        let formats = TabBar::new("identifiers.format")
            .segmented()
            .selected_index(self.format.index())
            .children(
                IdentifierFormat::ALL
                    .into_iter()
                    .map(|format| Tab::new().label(format.label())),
            )
            .on_click(cx.listener(|this, index, _window, cx| {
                if let Some(format) = IdentifierFormat::ALL.get(*index) {
                    this.set_format(*format, cx);
                }
            }));

        let mut format_controls = div().flex().flex_row().flex_wrap().items_center().gap_2();
        match self.format {
            IdentifierFormat::Uuid => {
                let versions = TabBar::new("identifiers.uuid-version")
                    .segmented()
                    .selected_index(self.version.index())
                    .children(
                        UuidVersion::ALL
                            .into_iter()
                            .map(|version| Tab::new().label(version.label())),
                    )
                    .on_click(cx.listener(|this, index, _window, cx| {
                        if let Some(version) = UuidVersion::ALL.get(*index) {
                            this.set_version(*version, cx);
                        }
                    }));
                format_controls = format_controls
                    .child(versions)
                    .child(
                        Button::new("identifiers.uppercase")
                            .label(if self.uppercase {
                                "Uppercase: on"
                            } else {
                                "Uppercase: off"
                            })
                            .when(self.uppercase, |button| button.primary())
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.toggle_uppercase(cx);
                            })),
                    )
                    .child(
                        Button::new("identifiers.hyphens")
                            .label(if self.hyphenated {
                                "Hyphens: on"
                            } else {
                                "Hyphens: off"
                            })
                            .when(self.hyphenated, |button| button.primary())
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.toggle_hyphens(cx);
                            })),
                    );
            }
            IdentifierFormat::Ulid => {
                format_controls = format_controls.child(
                    TabBar::new("identifiers.ulid-mode")
                        .segmented()
                        .selected_index(self.ulid_mode.index())
                        .children(
                            UlidMode::ALL
                                .into_iter()
                                .map(|mode| Tab::new().label(mode.label())),
                        )
                        .on_click(cx.listener(|this, index, _window, cx| {
                            if let Some(mode) = UlidMode::ALL.get(*index) {
                                this.set_ulid_mode(*mode, cx);
                            }
                        })),
                );
            }
            IdentifierFormat::Ksuid => {
                format_controls = format_controls.child(
                    Button::new("identifiers.ordered-ksuid")
                        .label(if self.ordered_ksuid {
                            "Ordered batch: on"
                        } else {
                            "Ordered batch: off"
                        })
                        .when(self.ordered_ksuid, |button| button.primary())
                        .on_click(cx.listener(|this, _event, _window, cx| {
                            this.toggle_ordered_ksuid(cx);
                        })),
                );
            }
        }

        let count_controls = div()
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
            .child(div().w_32().child(NumberInput::new(&self.count_input)));

        let toolbar = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Format"),
            )
            .child(formats)
            .child(format_controls)
            .child(count_controls)
            .child(
                Button::new("identifiers.generate")
                    .label("Generate")
                    .primary()
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.run_action(IdentifierAction::Generate, cx);
                    })),
            );

        let collision_note = div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child("Identifiers are collision-resistant, not guaranteed unique or secret.");

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(theme.background)
            .text_color(theme.foreground)
            .p_3()
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
                            .child("Identifier Generator"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("UUID, ULID and KSUID, local and offline"),
                    ),
            )
            .child(toolbar)
            .child(collision_note);

        if self.format == IdentifierFormat::Uuid && self.version.requires_namespace() {
            column = column.child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .w_full()
                    .child(ui::labeled_field(
                        cx,
                        "Namespace UUID",
                        None::<String>,
                        Input::new(&self.namespace)
                            .accessibility_id("identifiers.namespace")
                            .w_full(),
                    ))
                    .child(ui::labeled_field(
                        cx,
                        "Name",
                        None::<String>,
                        Input::new(&self.name)
                            .accessibility_id("identifiers.name")
                            .w_full(),
                    )),
            );
        }

        column = column.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .w_full()
                .child(ui::labeled_field(
                    cx,
                    "Inspect",
                    None::<String>,
                    Input::new(&self.inspect)
                        .accessibility_id("identifiers.inspect")
                        .w_full(),
                ))
                .child(
                    Button::new("identifiers.validate")
                        .label("Validate")
                        .on_click(cx.listener(|this, _event, _window, cx| {
                            this.run_action(IdentifierAction::Inspect, cx);
                        })),
                )
                .child(
                    Button::new("identifiers.paste")
                        .label("Paste")
                        .on_click(cx.listener(|this, _event, window, cx| {
                            this.paste_inspect(window, cx);
                        })),
                ),
        );

        let actions = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(ui::copy_feedback(
                cx,
                self.copied_all,
                "Copied all to Clipboard",
            ))
            .child(
                Button::new("identifiers.copy-all")
                    .icon(Icon::new(IconName::Copy))
                    .tooltip("Copy all identifiers")
                    .accessibility_label("Copy all identifiers")
                    .ghost()
                    .disabled(!can_copy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.copy_all(cx);
                    })),
            )
            .child(
                Button::new("identifiers.clear")
                    .label("Clear")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.clear(window, cx);
                    })),
            );
        column = column.child(actions);

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Identifier Generator History: {error}"),
                None,
            ));
        }

        let values_body = self.render_values(cx);
        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_4()
            .flex_1()
            .min_h_0()
            .child(ui::panel(
                cx,
                "Identifiers",
                "read-only generated values",
                values_body,
            ));
        if self.layout.read(cx).placement(UtilityId::Identifiers) == HistoryPlacement::Inline {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics(cx))
    }
}

/// Inputs and options that define a workspace session, excluding the generation
/// nonce and the derived previous-ULID carry so a repeat of the same settings
/// is not treated as different content.
#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkspaceSignature {
    format: IdentifierFormat,
    version: UuidVersion,
    count: u32,
    uppercase: bool,
    hyphenated: bool,
    ulid_mode: UlidMode,
    ordered_ksuid: bool,
    namespace: String,
    name: String,
    input: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RestoreDecision {
    Apply,
    Confirm,
}

/// A generated operation is different when either its controls or its
/// captured output differ. This protects repeated batches with identical
/// settings while allowing an exact restore to apply immediately.
fn restore_decision<C: PartialEq, O: PartialEq + ?Sized>(
    current_is_nonempty: bool,
    current_configuration: &C,
    restored_configuration: &C,
    current_output: &O,
    restored_output: &O,
) -> RestoreDecision {
    if current_is_nonempty
        && (current_configuration != restored_configuration || current_output != restored_output)
    {
        RestoreDecision::Confirm
    } else {
        RestoreDecision::Apply
    }
}

impl WorkspaceSignature {
    fn of(request: &IdentifiersRequest) -> Self {
        Self {
            format: request.format,
            version: request.version,
            count: request.count,
            uppercase: request.uppercase,
            hyphenated: request.hyphenated,
            ulid_mode: request.ulid_mode,
            ordered_ksuid: request.ordered_ksuid,
            namespace: request.namespace.clone(),
            name: request.name.clone(),
            input: request.input.clone(),
        }
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<IdentifiersSnapshot> {
    if entry.snapshot_version != Identifiers::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

fn preview_line(values: &[String]) -> String {
    let Some(first) = values.first() else {
        return "Empty result".to_owned();
    };
    let single = first.replace('\n', " ");
    let trimmed = single.trim();
    let mut preview: String = trimmed.chars().take(80).collect();
    if trimmed.chars().count() > 80 {
        preview.push('…');
    }
    if values.len() > 1 {
        format!("{preview} +{} more", values.len() - 1)
    } else {
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use gpui::{Focusable as _, VisualTestContext};
    use gpui_kit::component::Root;

    use crate::history::{HistoryClock, HistoryEntry, HistoryStore};
    use sofdevtool_core::utilities::identifiers::IdentifiersEvaluation;

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

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

    struct CountingClock(Rc<Cell<usize>>);

    impl HistoryClock for CountingClock {
        fn captured_at(&self) -> String {
            self.0.set(self.0.get() + 1);
            "2026-01-01T00:00:00Z".to_owned()
        }

        fn next_id(&self) -> String {
            self.0.set(self.0.get() + 1);
            format!("entry-{}", self.0.get())
        }
    }

    fn isolated_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-identifiers-restore-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn history_entry(id: &str, snapshot: IdentifiersSnapshot) -> HistoryEntry {
        HistoryEntry {
            id: id.to_owned(),
            captured_at: "2026-01-01T00:00:00Z".to_owned(),
            utility_id: Identifiers::ID.to_owned(),
            snapshot_version: Identifiers::SNAPSHOT_VERSION,
            payload: serde_json::to_value(snapshot).unwrap(),
        }
    }

    #[test]
    fn preview_line_summarizes_a_batch_without_losing_the_first_value() {
        assert_eq!(preview_line(&[]), "Empty result");
        assert_eq!(
            preview_line(&["5df41881-3aed-3515-88a7-2f4a814cf09e".to_owned()]),
            "5df41881-3aed-3515-88a7-2f4a814cf09e"
        );
        assert_eq!(
            preview_line(&["a".to_owned(), "b".to_owned(), "c".to_owned()]),
            "a +2 more"
        );
    }

    #[test]
    fn workspace_signature_ignores_the_generation_nonce() {
        let request = IdentifiersRequest {
            generation: 7,
            ..IdentifiersRequest::default()
        };
        let first = WorkspaceSignature::of(&request);
        let second = WorkspaceSignature::of(&IdentifiersRequest {
            generation: 99,
            ..request
        });
        assert_eq!(first, second);
    }

    #[test]
    fn workspace_signature_ignores_the_derived_ulid_carry_but_not_the_format_controls() {
        let request = IdentifiersRequest {
            format: IdentifierFormat::Ulid,
            ulid_mode: UlidMode::Monotonic,
            previous_ulid: Some([1u8; 16]),
            generation: 3,
            ..IdentifiersRequest::default()
        };
        let carried = WorkspaceSignature::of(&IdentifiersRequest {
            previous_ulid: Some([9u8; 16]),
            ..request.clone()
        });
        assert_eq!(WorkspaceSignature::of(&request), carried);

        let other_mode = WorkspaceSignature::of(&IdentifiersRequest {
            ulid_mode: UlidMode::Random,
            ..request.clone()
        });
        assert_ne!(WorkspaceSignature::of(&request), other_mode);
    }

    #[test]
    fn different_generated_batches_with_identical_controls_require_confirmation() {
        let request = IdentifiersRequest::default();
        let signature = WorkspaceSignature::of(&request);
        let decision = restore_decision(
            true,
            &signature,
            &signature,
            &["current-batch".to_owned()][..],
            &["captured-batch".to_owned()][..],
        );
        assert_eq!(decision, RestoreDecision::Confirm);
    }

    #[test]
    fn empty_and_equivalent_identifier_sessions_apply_without_confirmation() {
        let request = IdentifiersRequest::default();
        let signature = WorkspaceSignature::of(&request);
        assert_eq!(
            restore_decision(
                false,
                &signature,
                &signature,
                &[] as &[String],
                &["saved".to_owned()][..]
            ),
            RestoreDecision::Apply,
        );
        assert_eq!(
            restore_decision(
                true,
                &signature,
                &signature,
                &["same".to_owned()][..],
                &["same".to_owned()][..]
            ),
            RestoreDecision::Apply,
        );
    }

    #[gpui::test]
    fn workspace_restore_confirmation_cancel_and_confirm_use_the_captured_batch(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let clock_calls = Rc::new(Cell::new(0));
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(CountingClock(clock_calls.clone())),
        ));
        let snapshot = IdentifiersSnapshot {
            request: IdentifiersRequest {
                generation: 7,
                ..IdentifiersRequest::default()
            },
            values: vec![
                "captured-identifier-a".to_owned(),
                "captured-identifier-b".to_owned(),
            ],
        };
        let entry = history_entry("captured", snapshot.clone());
        history.store().record(entry.clone()).unwrap();
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| IdentifiersWorkspace::new(window, cx, clipboard, history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                let request = IdentifiersRequest {
                    generation: 8,
                    ..IdentifiersRequest::default()
                };
                let SubmitOutcome::Scheduled(revision) = view.session.submit(request.clone())
                else {
                    panic!("initial current batch must schedule");
                };
                assert!(view
                    .session
                    .publish(
                        revision,
                        IdentifiersEvaluation::Valid {
                            values: vec!["current-identifier".to_owned()],
                        },
                    )
                    .is_some());
                let current_evaluation = view.session.evaluation().clone();
                let current_request = view.session.request().unwrap().clone();
                let initial_generation = view.generation;

                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.history_view.pending_restore, Some(entry.clone()));
                view.cancel_restore(cx);
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(view.session.request(), Some(&current_request));
                assert_eq!(view.session.evaluation(), &current_evaluation);

                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.history_view.pending_restore, Some(entry.clone()));
                view.confirm_restore(window, cx);
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(view.session.evaluation().values(), snapshot.values);
                assert_eq!(
                    view.signature(cx),
                    WorkspaceSignature::of(&snapshot.request)
                );
                assert_eq!(view.generation, initial_generation.max(7));
                assert!(view.session.take_snapshot().is_none());
            });
            assert_eq!(history.load(Identifiers::ID).unwrap().len(), 1);
            workspace.update(cx, |view, _| {
                assert!(view.history_view.select(&entry.id));
                view.history_view.pending_restore = Some(entry.clone());
            });
            history.clear_utility(Identifiers::ID, cx).unwrap();
            let view = workspace.read(cx);
            assert!(view.history_view.entries.is_empty());
            assert!(view.history_view.selected.is_none());
            assert!(view.history_view.pending_restore.is_none());
            assert_eq!(view.session.evaluation().values(), snapshot.values);
            workspace.update(cx, |view, cx| {
                view.history_view.pending_restore = Some(entry.clone());
                view.confirm_restore(window, cx);
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(view.session.evaluation().values(), snapshot.values);
            });
            assert_eq!(clock_calls.get(), 0);
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn empty_and_equivalent_identifier_workspaces_restore_without_confirmation(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let clock_calls = Rc::new(Cell::new(0));
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(CountingClock(clock_calls.clone())),
        ));
        let snapshot = IdentifiersSnapshot {
            request: IdentifiersRequest::default(),
            values: vec!["captured-identifier".to_owned()],
        };
        let entry = history_entry("captured", snapshot.clone());
        history.store().record(entry.clone()).unwrap();
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| IdentifiersWorkspace::new(window, cx, clipboard, history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.request_restore(entry.clone(), window, cx);
                assert!(
                    view.history_view.pending_restore.is_none(),
                    "empty sessions apply directly"
                );
                assert_eq!(view.session.evaluation().values(), snapshot.values);
                view.request_restore(entry.clone(), window, cx);
                assert!(
                    view.history_view.pending_restore.is_none(),
                    "equivalent sessions apply directly"
                );
            });
            assert_eq!(history.load(Identifiers::ID).unwrap().len(), 1);
            assert_eq!(clock_calls.get(), 0);
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn segmented_modes_repeat_generation_and_copy_actions_preserve_exact_values(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(crate::history::SystemClock::new()),
        ));
        let clipboard = Rc::new(TestClipboard::default());
        let app_clipboard: Rc<dyn Clipboard> = clipboard.clone();
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| IdentifiersWorkspace::new(window, cx, app_clipboard, history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.set_format(IdentifierFormat::Ulid, cx);
                assert_eq!(view.format, IdentifierFormat::Ulid);
                view.set_ulid_mode(UlidMode::Monotonic, cx);
                assert_eq!(view.ulid_mode, UlidMode::Monotonic);
                view.set_format(IdentifierFormat::Uuid, cx);
                view.set_version(UuidVersion::V7, cx);
                assert_eq!(view.version, UuidVersion::V7);
            });
            window.draw(cx).clear(cx);
        });

        // The kit number input owns stepping: ArrowUp/ArrowDown reach the
        // workspace through the same Change path as the stepper buttons.
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).count_input.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("up");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(workspace.read_with(&cx, |view, _| view.count), 2);
        cx.simulate_keystrokes("down");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(workspace.read_with(&cx, |view, _| view.count), 1);

        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| {
                view.run_action(IdentifierAction::Generate, cx);
                view.run_action(IdentifierAction::Generate, cx);
            });
        });
        let values =
            workspace.read_with(&cx, |view, _| view.session.evaluation().values().to_vec());
        assert_eq!(values.len(), 1);
        assert_eq!(history.load(Identifiers::ID).unwrap().len(), 2);

        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_value(0, cx));
        });
        assert_eq!(clipboard.0.borrow().as_deref(), Some(values[0].as_str()));

        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_all(cx));
        });
        assert_eq!(
            clipboard.0.borrow().as_deref(),
            Some(values.join("\n").as_str())
        );
        assert_eq!(history.load(Identifiers::ID).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
