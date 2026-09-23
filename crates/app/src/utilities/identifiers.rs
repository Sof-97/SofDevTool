//! The Identifier Generator workspace: explicit UUID generation and inspection
//! with version-specific controls, per-item Clipboard actions and fresh Rust
//! History. Generation is never implicit: every Generate or Validate is a
//! deliberate action that records its own settled operation.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, AnyElement, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::identifiers::{
    decode_ulid, IdentifierAction, IdentifierFormat, Identifiers, IdentifiersRequest,
    IdentifiersSnapshot, UlidMode, UuidVersion, DEFAULT_NAMESPACE, MAXIMUM_GENERATED_COUNT,
    MAXIMUM_ORDERED_KSUID_COUNT,
};
use sofdevtool_core::utility::Utility;
use sofui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    ConfirmationBar, DiagnosticSeverity, LabeledField, NumericStepper, SegmentedControl,
    SegmentedControlFocus, SegmentedOption, SelectableList, SelectableListFocus, SelectableRow,
    TextField, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{
    HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState, RETENTION,
};
use crate::workbench::Workbench;

type IdentifiersSession = Session<Identifiers>;

/// Builds the Identifier Generator workspace as a type-erased view for the
/// Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| IdentifiersWorkspace::new(window, cx, clipboard, history))
        .into()
}

/// One distinct focus handle per simultaneously-rendered button. Reusing a
/// handle across two visible buttons aborts GPUI when both request focus in a
/// single frame.
struct ButtonFocus {
    ordered_ksuid: FocusHandle,
    uppercase: FocusHandle,
    hyphens: FocusHandle,
    count_stepper: [FocusHandle; 2],
    generate: FocusHandle,
    validate: FocusHandle,
    paste: FocusHandle,
    copy_all: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct IdentifiersWorkspace {
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    format: IdentifierFormat,
    version: UuidVersion,
    count: u32,
    uppercase: bool,
    hyphenated: bool,
    ulid_mode: UlidMode,
    ordered_ksuid: bool,
    previous_ulid: Option<[u8; 16]>,
    namespace: TextField,
    name: TextField,
    inspect: TextField,
    session: IdentifiersSession,
    generation: u64,
    copied_all: bool,
    copied_index: Option<usize>,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_focus: SelectableListFocus,
    selection_focus: SegmentedControlFocus,
    history_visible: bool,
    _history_subscription: HistorySubscription,
    copy_focus: Vec<FocusHandle>,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl IdentifiersWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let namespace = TextField::new(window, cx);
        let name = TextField::new(window, cx);
        let inspect = TextField::new(window, cx);
        namespace.assign_text(DEFAULT_NAMESPACE, window, cx);
        let subscriptions = vec![
            namespace.on_change_in(window, cx, |this, _window, cx| this.invalidate(cx)),
            name.on_change_in(window, cx, |this, _window, cx| this.invalidate(cx)),
            inspect.on_change_in(window, cx, |this, _window, cx| this.invalidate(cx)),
        ];
        let history_view = HistoryViewState::load(&history, Identifiers::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Identifiers::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        Self {
            clipboard,
            history,
            format: IdentifierFormat::Uuid,
            version: UuidVersion::V4,
            count: 1,
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
            history_focus: SelectableListFocus::new(),
            selection_focus: SegmentedControlFocus::new(),
            history_visible: true,
            _history_subscription: history_subscription,
            copy_focus: Vec::new(),
            focus: ButtonFocus {
                ordered_ksuid: cx.focus_handle().tab_stop(true).tab_index(0),
                uppercase: cx.focus_handle().tab_stop(true).tab_index(0),
                hyphens: cx.focus_handle().tab_stop(true).tab_index(0),
                count_stepper: std::array::from_fn(|_| {
                    cx.focus_handle().tab_stop(true).tab_index(0)
                }),
                generate: cx.focus_handle().tab_stop(true).tab_index(0),
                validate: cx.focus_handle().tab_stop(true).tab_index(0),
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

    fn request(&self, action: IdentifierAction, cx: &App) -> IdentifiersRequest {
        IdentifiersRequest {
            action,
            format: self.format,
            version: self.version,
            count: self.count,
            uppercase: self.uppercase,
            hyphenated: self.hyphenated,
            namespace: self.namespace.text(cx),
            name: self.name.text(cx),
            input: self.inspect.text(cx),
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
            namespace: self.namespace.text(cx),
            name: self.name.text(cx),
            input: self.inspect.text(cx),
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
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Identifiers::ID);
        cx.notify();
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

    fn adjust_count(&mut self, delta: i64, cx: &mut Context<Self>) {
        let next = (i64::from(self.count) + delta).clamp(1, i64::from(self.maximum_count()));
        let next = next as u32;
        if next != self.count {
            self.count = next;
            self.invalidate(cx);
        }
    }

    fn paste_inspect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.suppress_changes = true;
            self.inspect.edit_text(text, window, cx);
            self.suppress_changes = false;
            self.invalidate(cx);
        }
    }

    fn copy_value(&mut self, index: usize, value: String, cx: &mut Context<Self>) {
        self.clipboard.write_text(&value, cx);
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
        self.inspect.edit_text("", window, cx);
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
        self.namespace
            .assign_text(snapshot.request.namespace.clone(), window, cx);
        self.name
            .assign_text(snapshot.request.name.clone(), window, cx);
        self.inspect
            .assign_text(snapshot.request.input.clone(), window, cx);
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
                        .map(|snapshot| preview_line(&snapshot.values))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: snapshot.is_none().then(|| "Unavailable".to_owned()),
                    selectable: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn ensure_copy_focus(&mut self, count: usize, cx: &mut Context<Self>) {
        while self.copy_focus.len() < count {
            self.copy_focus
                .push(cx.focus_handle().tab_stop(true).tab_index(0));
        }
    }

    fn render_values(&self, values: &[String], cx: &mut Context<Self>) -> AnyElement {
        let tokens = ThemeTokens::active();
        if values.is_empty() {
            return empty_state("Generate identifiers or validate one to begin").into_any_element();
        }
        let mut list = div()
            .id("identifiers.results")
            .flex()
            .flex_col()
            .gap_1()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        for (index, value) in values.iter().enumerate() {
            let display = value.clone();
            let to_copy = value.clone();
            list = list.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .w_full()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(tokens.border())
                    .bg(tokens.surface_raised())
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .text_color(tokens.text())
                            .child(display),
                    )
                    .child(copy_feedback(self.copied_index == Some(index), "Copied"))
                    .child(
                        Button::with_id(
                            format!("identifiers.copy.{index}"),
                            format!("Copy {}", index + 1),
                        )
                        .focus_handle(self.copy_focus[index].clone())
                        .on_click(view_click(
                            cx,
                            move |this, _window, cx| {
                                this.copy_value(index, to_copy.clone(), cx);
                            },
                        )),
                    ),
            );
        }
        list.into_any_element()
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
            Button::with_id("identifiers.history.restore-selected", "Restore selected")
                .disabled(!restore_enabled)
                .focus_handle(self.focus.history_restore.clone())
                .on_click(view_click(cx, |this, window, cx| {
                    this.restore_selected(window, cx);
                })),
        );
        let weak = cx.weak_entity();
        let panel = SelectableList::new(
            "identifiers.history",
            "History",
            self.history_items(),
            selected,
            "No retained operations yet.",
            self.history_focus.clone(),
        )
        .summary(format!("{}/{}", self.history_view.entries.len(), RETENTION))
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
        ConfirmationBar::new(
            "identifiers.restore-confirmation",
            "Restoring this entry replaces the current non-empty Identifier Generator session.",
            "Restore",
            "Cancel",
        )
        .focus_handles(
            self.focus.history_confirm.clone(),
            self.focus.history_cancel.clone(),
        )
        .on_confirm(view_click(cx, |this, window, cx| {
            this.confirm_restore(window, cx);
        }))
        .on_cancel(view_click(cx, |this, _window, cx| {
            this.cancel_restore(cx);
        }))
    }
}

impl Render for IdentifiersWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let values = self.session.evaluation().values().to_vec();
        self.ensure_copy_focus(values.len(), cx);
        let can_copy = !values.is_empty();

        let formats = SegmentedControl::new(
            "identifiers.format",
            "Format",
            IdentifierFormat::ALL
                .into_iter()
                .map(|format| SegmentedOption::new(format_id(format), format.label()))
                .collect(),
            Some(format_id(self.format).to_owned()),
            self.selection_focus.clone(),
        )
        .on_change({
            let weak = cx.weak_entity();
            Rc::new(move |id, _window, cx| {
                weak.update(cx, |this, cx| {
                    let format = match id {
                        "uuid" => IdentifierFormat::Uuid,
                        "ulid" => IdentifierFormat::Ulid,
                        "ksuid" => IdentifierFormat::Ksuid,
                        _ => return,
                    };
                    this.set_format(format, cx);
                })
                .ok();
            })
        });

        // Only the controls that belong to the active format are rendered.
        let mut format_controls = div().flex().flex_row().flex_wrap().items_center().gap_2();
        match self.format {
            IdentifierFormat::Uuid => {
                let versions = SegmentedControl::new(
                    "identifiers.uuid-version",
                    "UUID version",
                    UuidVersion::ALL
                        .into_iter()
                        .map(|version| SegmentedOption::new(version_id(version), version.label()))
                        .collect(),
                    Some(version_id(self.version).to_owned()),
                    self.selection_focus.clone(),
                )
                .on_change({
                    let weak = cx.weak_entity();
                    Rc::new(move |id, _window, cx| {
                        weak.update(cx, |this, cx| {
                            let version = match id {
                                "v1" => UuidVersion::V1,
                                "v3" => UuidVersion::V3,
                                "v4" => UuidVersion::V4,
                                "v5" => UuidVersion::V5,
                                "v6" => UuidVersion::V6,
                                "v7" => UuidVersion::V7,
                                _ => return,
                            };
                            this.set_version(version, cx);
                        })
                        .ok();
                    })
                });
                format_controls = format_controls
                    .child(versions)
                    .child(
                        Button::with_id(
                            "identifiers.uppercase",
                            if self.uppercase {
                                "Uppercase: on"
                            } else {
                                "Uppercase: off"
                            },
                        )
                        .variant(if self.uppercase {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Secondary
                        })
                        .focus_handle(self.focus.uppercase.clone())
                        .on_click(view_click(cx, |this, _window, cx| {
                            this.toggle_uppercase(cx);
                        })),
                    )
                    .child(
                        Button::with_id(
                            "identifiers.hyphens",
                            if self.hyphenated {
                                "Hyphens: on"
                            } else {
                                "Hyphens: off"
                            },
                        )
                        .variant(if self.hyphenated {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Secondary
                        })
                        .focus_handle(self.focus.hyphens.clone())
                        .on_click(view_click(cx, |this, _window, cx| {
                            this.toggle_hyphens(cx);
                        })),
                    );
            }
            IdentifierFormat::Ulid => {
                format_controls = format_controls.child(
                    SegmentedControl::new(
                        "identifiers.ulid-mode",
                        "ULID mode",
                        UlidMode::ALL
                            .into_iter()
                            .map(|mode| SegmentedOption::new(ulid_mode_id(mode), mode.label()))
                            .collect(),
                        Some(ulid_mode_id(self.ulid_mode).to_owned()),
                        self.selection_focus.clone(),
                    )
                    .on_change({
                        let weak = cx.weak_entity();
                        Rc::new(move |id, _window, cx| {
                            weak.update(cx, |this, cx| {
                                let mode = match id {
                                    "random" => UlidMode::Random,
                                    "monotonic" => UlidMode::Monotonic,
                                    _ => return,
                                };
                                this.set_ulid_mode(mode, cx);
                            })
                            .ok();
                        })
                    }),
                );
            }
            IdentifierFormat::Ksuid => {
                format_controls = format_controls.child(
                    Button::with_id(
                        "identifiers.ordered-ksuid",
                        if self.ordered_ksuid {
                            "Ordered batch: on"
                        } else {
                            "Ordered batch: off"
                        },
                    )
                    .variant(if self.ordered_ksuid {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Secondary
                    })
                    .focus_handle(self.focus.ordered_ksuid.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.toggle_ordered_ksuid(cx);
                    })),
                );
            }
        }

        let maximum = self.maximum_count();
        let count_controls = NumericStepper::new(
            "identifiers.count",
            "Count",
            Some(self.count as i32),
            1,
            maximum as i32,
            1,
        )
        .focus_handles(
            self.focus.count_stepper[0].clone(),
            self.focus.count_stepper[1].clone(),
        )
        .on_step({
            let weak = cx.weak_entity();
            move |delta, _window, cx| {
                weak.update(cx, |this, cx| this.adjust_count(i64::from(delta), cx))
                    .ok();
            }
        });

        let toolbar = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Format"),
            )
            .child(formats)
            .child(format_controls)
            .child(count_controls)
            .child(
                Button::primary_with_id("identifiers.generate", "Generate")
                    .focus_handle(self.focus.generate.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.run_action(IdentifierAction::Generate, cx);
                    })),
            );

        let collision_note = div()
            .text_xs()
            .text_color(tokens.text_muted())
            .child("Identifiers are collision-resistant, not guaranteed unique or secret.");

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(tokens.background())
            .text_color(tokens.text())
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
                            .text_color(tokens.text_muted())
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
                    .child(LabeledField::new(
                        "Namespace UUID",
                        self.namespace.render("identifiers.namespace"),
                    ))
                    .child(LabeledField::new(
                        "Name",
                        self.name.render("identifiers.name"),
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
                .child(LabeledField::new(
                    "Inspect",
                    self.inspect.render("identifiers.inspect"),
                ))
                .child(
                    Button::with_id("identifiers.validate", "Validate")
                        .focus_handle(self.focus.validate.clone())
                        .on_click(view_click(cx, |this, _window, cx| {
                            this.run_action(IdentifierAction::Inspect, cx);
                        })),
                )
                .child(
                    Button::with_id("identifiers.paste", "Paste")
                        .focus_handle(self.focus.paste.clone())
                        .on_click(view_click(cx, |this, window, cx| {
                            this.paste_inspect(window, cx);
                        })),
                ),
        );

        let actions = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(copy_feedback(self.copied_all, "Copied all to Clipboard"))
            .child(
                Button::with_id(
                    "identifiers.history.toggle",
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
                Button::with_id("identifiers.copy-all", "Copy All")
                    .disabled(!can_copy)
                    .focus_handle(self.focus.copy_all.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.copy_all(cx);
                    })),
            )
            .child(
                Button::with_id("identifiers.clear", "Clear")
                    .focus_handle(self.focus.clear.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.clear(window, cx);
                    })),
            );
        column = column.child(actions);

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("Identifier Generator History: {error}"),
                None,
            ));
        }

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_4()
            .flex_1()
            .min_h_0()
            .child(panel(
                "Identifiers",
                "read-only generated values",
                self.render_values(&values, cx),
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics())
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

fn format_id(format: IdentifierFormat) -> &'static str {
    match format {
        IdentifierFormat::Uuid => "uuid",
        IdentifierFormat::Ulid => "ulid",
        IdentifierFormat::Ksuid => "ksuid",
    }
}

fn version_id(version: UuidVersion) -> &'static str {
    match version {
        UuidVersion::V1 => "v1",
        UuidVersion::V3 => "v3",
        UuidVersion::V4 => "v4",
        UuidVersion::V5 => "v5",
        UuidVersion::V6 => "v6",
        UuidVersion::V7 => "v7",
    }
}

fn ulid_mode_id(mode: UlidMode) -> &'static str {
    match mode {
        UlidMode::Random => "random",
        UlidMode::Monotonic => "monotonic",
    }
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

    use crate::history::{HistoryClock, HistoryEntry, HistoryStore};
    use sofdevtool_core::utilities::identifiers::IdentifiersEvaluation;

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
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
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
        cx.update(sofui::init);
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
        let (workspace, cx) = cx.add_window_view(|window, cx| {
            IdentifiersWorkspace::new(window, cx, clipboard, history.clone())
        });

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
        cx.update(sofui::init);
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
        let (workspace, cx) = cx.add_window_view(|window, cx| {
            IdentifiersWorkspace::new(window, cx, clipboard, history.clone())
        });

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
        cx.update(sofui::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(crate::history::SystemClock::new()),
        ));
        let clipboard = Rc::new(TestClipboard::default());
        let app_clipboard: Rc<dyn Clipboard> = clipboard.clone();
        let (workspace, cx) = cx.add_window_view(|window, cx| {
            IdentifiersWorkspace::new(window, cx, app_clipboard, history.clone())
        });

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let selection_focus = workspace.read(cx).selection_focus.clone();
            let focus = selection_focus.handle("identifiers.format", "ulid", cx);
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(cx, |view, _| view.format),
            IdentifierFormat::Ulid
        );

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let selection_focus = workspace.read(cx).selection_focus.clone();
            let focus = selection_focus.handle("identifiers.ulid-mode", "monotonic", cx);
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(cx, |view, _| view.ulid_mode),
            UlidMode::Monotonic
        );

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let selection_focus = workspace.read(cx).selection_focus.clone();
            let focus = selection_focus.handle("identifiers.format", "uuid", cx);
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let selection_focus = workspace.read(cx).selection_focus.clone();
            let focus = selection_focus.handle("identifiers.uuid-version", "v7", cx);
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(cx, |view, _| view.version),
            UuidVersion::V7
        );

        for _ in 0..2 {
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                let focus = workspace.read(cx).focus.generate.clone();
                window.focus(&focus, cx);
            });
            cx.simulate_keystrokes("enter");
        }
        let values = workspace.read_with(cx, |view, _| view.session.evaluation().values().to_vec());
        assert_eq!(values.len(), 1);
        assert_eq!(history.load(Identifiers::ID).unwrap().len(), 2);

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).copy_focus[0].clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().as_deref(), Some(values[0].as_str()));

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.copy_all.clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            clipboard.0.borrow().as_deref(),
            Some(values.join("\n").as_str())
        );
        assert_eq!(history.load(Identifiers::ID).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
