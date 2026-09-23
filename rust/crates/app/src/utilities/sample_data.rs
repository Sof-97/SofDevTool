//! The Sample Data workspace: edit a reordered, typed field list, choose JSON
//! or CSV and generate fictional rows on an explicit action. Output is
//! read-only; restoring a History entry rebuilds the exact schema and output
//! without regenerating.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::sample_data::{
    FieldDefinition, SampleData, SampleDataEvaluation, SampleDataFormat, SampleDataRequest,
    SampleDataSnapshot, SampleFieldType, MAXIMUM_FIELD_COUNT, MAXIMUM_ROW_COUNT, MINIMUM_ROW_COUNT,
};
use sofdevtool_core::utility::Utility;
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    DiagnosticSeverity, HistoryItem, HistoryPanel, LabeledField, TextEditor, TextField,
    ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder};
use crate::workbench::Workbench;

type SampleDataSession = Session<SampleData>;

/// Builds the Sample Data workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| SampleDataWorkspace::new(window, cx, clipboard, history))
        .into()
}

/// Distinct focus handles for buttons that persist across renders. Field-local
/// buttons own their handles in [`FieldState`].
struct ButtonFocus {
    json: FocusHandle,
    csv: FocusHandle,
    row_decrement: FocusHandle,
    row_increment: FocusHandle,
    add_field: FocusHandle,
    generate: FocusHandle,
    clear: FocusHandle,
    copy: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

/// Per-field focus handles. Each simultaneously rendered button needs its own
/// handle and an explicit element id, or GPUI aborts on a duplicate a11y node.
struct FieldFocus {
    cycle_type: FocusHandle,
    toggle_integer: FocusHandle,
    up: FocusHandle,
    down: FocusHandle,
    remove: FocusHandle,
}

/// The editable state of one field, including its own text fields so reordering
/// moves the whole editor and removal drops its subscriptions.
struct FieldState {
    field_type: SampleFieldType,
    number_is_integer: bool,
    name: TextField,
    number_minimum: TextField,
    number_maximum: TextField,
    date_minimum: TextField,
    date_maximum: TextField,
    choices: TextField,
    focus: FieldFocus,
    _subscriptions: Vec<Subscription>,
}

impl FieldState {
    fn new(
        window: &mut Window,
        cx: &mut Context<SampleDataWorkspace>,
        definition: &FieldDefinition,
    ) -> Self {
        let name = TextField::new(window, cx);
        let number_minimum = TextField::new(window, cx);
        let number_maximum = TextField::new(window, cx);
        let date_minimum = TextField::new(window, cx);
        let date_maximum = TextField::new(window, cx);
        let choices = TextField::new(window, cx);
        name.set_text(definition.name.clone(), window, cx);
        number_minimum.set_text(number_text(definition.number_minimum), window, cx);
        number_maximum.set_text(number_text(definition.number_maximum), window, cx);
        date_minimum.set_text(number_text(definition.date_minimum_seconds), window, cx);
        date_maximum.set_text(number_text(definition.date_maximum_seconds), window, cx);
        choices.set_text(definition.enum_choices.join(","), window, cx);

        let subscriptions = vec![
            name.on_change_in(window, cx, |this, _window, cx| this.invalidate(cx)),
            number_minimum.on_change_in(window, cx, |this, _window, cx| this.invalidate(cx)),
            number_maximum.on_change_in(window, cx, |this, _window, cx| this.invalidate(cx)),
            date_minimum.on_change_in(window, cx, |this, _window, cx| this.invalidate(cx)),
            date_maximum.on_change_in(window, cx, |this, _window, cx| this.invalidate(cx)),
            choices.on_change_in(window, cx, |this, _window, cx| this.invalidate(cx)),
        ];

        Self {
            field_type: definition.field_type,
            number_is_integer: definition.number_is_integer,
            name,
            number_minimum,
            number_maximum,
            date_minimum,
            date_maximum,
            choices,
            focus: FieldFocus {
                cycle_type: cx.focus_handle().tab_stop(true).tab_index(0),
                toggle_integer: cx.focus_handle().tab_stop(true).tab_index(0),
                up: cx.focus_handle().tab_stop(true).tab_index(0),
                down: cx.focus_handle().tab_stop(true).tab_index(0),
                remove: cx.focus_handle().tab_stop(true).tab_index(0),
            },
            _subscriptions: subscriptions,
        }
    }

    fn definition(&self, cx: &App) -> FieldDefinition {
        FieldDefinition {
            name: self.name.text(cx),
            field_type: self.field_type,
            number_minimum: parse_number(&self.number_minimum.text(cx)),
            number_maximum: parse_number(&self.number_maximum.text(cx)),
            number_is_integer: self.number_is_integer,
            date_minimum_seconds: parse_number(&self.date_minimum.text(cx)),
            date_maximum_seconds: parse_number(&self.date_maximum.text(cx)),
            enum_choices: self
                .choices
                .text(cx)
                .split(',')
                .map(str::to_owned)
                .collect(),
        }
    }
}

pub struct SampleDataWorkspace {
    fields: Vec<FieldState>,
    row_count: u32,
    output_format: SampleDataFormat,
    session: SampleDataSession,
    generation: u64,
    display_epoch: u64,
    result: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    copied: bool,
    suppress_changes: bool,
    history_entries: Vec<HistoryEntry>,
    history_selected: Option<String>,
    history_visible: bool,
    history_error: Option<String>,
    pending_restore: Option<HistoryEntry>,
    focus: ButtonFocus,
}

impl SampleDataWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let default = SampleDataRequest::default();
        let fields = default
            .fields
            .iter()
            .map(|definition| FieldState::new(window, cx, definition))
            .collect();
        let result = TextEditor::new(window, cx);
        let (history_entries, history_error) = match history.load(SampleData::ID) {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        Self {
            fields,
            row_count: default.row_count,
            output_format: default.output,
            session: SampleDataSession::new(),
            generation: 0,
            display_epoch: u64::MAX,
            result,
            clipboard,
            history,
            copied: false,
            suppress_changes: false,
            history_entries,
            history_selected: None,
            history_visible: true,
            history_error,
            pending_restore: None,
            focus: ButtonFocus {
                json: cx.focus_handle().tab_stop(true).tab_index(0),
                csv: cx.focus_handle().tab_stop(true).tab_index(0),
                row_decrement: cx.focus_handle().tab_stop(true).tab_index(0),
                row_increment: cx.focus_handle().tab_stop(true).tab_index(0),
                add_field: cx.focus_handle().tab_stop(true).tab_index(0),
                generate: cx.focus_handle().tab_stop(true).tab_index(0),
                clear: cx.focus_handle().tab_stop(true).tab_index(0),
                copy: cx.focus_handle().tab_stop(true).tab_index(0),
                history_toggle: cx.focus_handle().tab_stop(true).tab_index(0),
                history_restore: cx.focus_handle().tab_stop(true).tab_index(0),
                history_confirm: cx.focus_handle().tab_stop(true).tab_index(0),
                history_cancel: cx.focus_handle().tab_stop(true).tab_index(0),
            },
        }
    }

    fn request(&self, cx: &App) -> SampleDataRequest {
        SampleDataRequest {
            fields: self
                .fields
                .iter()
                .map(|field| field.definition(cx))
                .collect(),
            row_count: self.row_count,
            output: self.output_format,
            generation: self.generation,
        }
    }

    fn signature(&self, cx: &App) -> SampleDataRequest {
        self.request(cx).configuration()
    }

    /// Clears stale visible output after any edit. Nothing is regenerated until
    /// the next explicit Generate.
    fn invalidate(&mut self, cx: &mut Context<Self>) {
        if self.suppress_changes {
            return;
        }
        self.session.clear();
        self.copied = false;
        cx.notify();
    }

    fn generate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.generation = self.generation.wrapping_add(1);
        let request = self.request(cx);
        let SubmitOutcome::Scheduled(revision) = self.session.submit(request) else {
            return;
        };
        self.session.resolve(revision);
        self.copied = false;
        self.record_settled(cx);
        self.sync_display(window, cx);
        cx.notify();
    }

    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a Sample Data snapshot serializes");
        match self
            .history
            .record(SampleData::ID, SampleData::SNAPSHOT_VERSION, payload)
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

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        let output = self
            .session
            .evaluation()
            .output()
            .unwrap_or_default()
            .to_owned();
        self.result.set_text(output, window, cx);
    }

    fn set_output_format(&mut self, format: SampleDataFormat, cx: &mut Context<Self>) {
        if self.output_format != format {
            self.output_format = format;
            self.invalidate(cx);
        }
    }

    fn adjust_row_count(&mut self, delta: i64, cx: &mut Context<Self>) {
        let next = (i64::from(self.row_count) + delta)
            .clamp(i64::from(MINIMUM_ROW_COUNT), i64::from(MAXIMUM_ROW_COUNT))
            as u32;
        if next != self.row_count {
            self.row_count = next;
            self.invalidate(cx);
        }
    }

    fn add_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.fields.len() >= MAXIMUM_FIELD_COUNT {
            return;
        }
        let index = self.fields.len();
        let definition =
            FieldDefinition::new(format!("field{}", index + 1), SampleFieldType::Enumeration);
        self.fields.push(FieldState::new(window, cx, &definition));
        self.invalidate(cx);
    }

    fn remove_field(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.fields.len() {
            self.fields.remove(index);
            self.invalidate(cx);
        }
    }

    fn move_field(&mut self, index: usize, delta: i64, cx: &mut Context<Self>) {
        let target = index as i64 + delta;
        if target < 0 || target >= self.fields.len() as i64 {
            return;
        }
        self.fields.swap(index, target as usize);
        self.invalidate(cx);
    }

    fn cycle_field_type(&mut self, index: usize, cx: &mut Context<Self>) {
        let next = self
            .fields
            .get(index)
            .map(|field| field.field_type.successor());
        if let Some(next) = next {
            self.fields[index].field_type = next;
            self.invalidate(cx);
        }
    }

    fn toggle_integer(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(field) = self.fields.get_mut(index) {
            field.number_is_integer = !field.number_is_integer;
        }
        self.invalidate(cx);
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.session.clear();
        self.copied = false;
        self.sync_display(window, cx);
        cx.notify();
    }

    fn copy_result(&mut self, cx: &mut Context<Self>) {
        if let Some(output) = self.session.evaluation().output() {
            let output = output.to_owned();
            self.clipboard.write_text(&output, cx);
            self.copied = true;
            cx.notify();
        }
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
        let current_configuration = self.signature(cx);
        let baseline_configuration = SampleDataRequest::default().configuration();
        let nonempty = self.session.evaluation().is_valid_operation()
            || current_configuration != baseline_configuration;
        if restore_decision(
            nonempty,
            &current_configuration,
            &snapshot.request.configuration(),
            &self.session.evaluation().output(),
            &Some(snapshot.output.as_str()),
        ) == RestoreDecision::Confirm
        {
            self.pending_restore = Some(entry);
            cx.notify();
        } else {
            self.apply_restore(snapshot, window, cx);
        }
    }

    fn apply_restore(
        &mut self,
        snapshot: SampleDataSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        let request = snapshot.request.clone();
        self.fields = request
            .fields
            .iter()
            .map(|definition| FieldState::new(window, cx, definition))
            .collect();
        self.row_count = request.row_count;
        self.output_format = request.output;
        self.session.restore(snapshot);
        self.generation = self.generation.max(
            self.session
                .request()
                .map_or(0, |request| request.generation),
        );
        self.suppress_changes = false;
        self.pending_restore = None;
        self.copied = false;
        self.display_epoch = u64::MAX;
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
                        .map(|snapshot| preview_line(&snapshot.output))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn format_button(&self, format: SampleDataFormat, cx: &mut Context<Self>) -> Button {
        Button::new(format.label())
            .variant(if self.output_format == format {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(match format {
                SampleDataFormat::Json => self.focus.json.clone(),
                SampleDataFormat::Csv => self.focus.csv.clone(),
            })
            .on_click(view_click(cx, move |this, _window, cx| {
                this.set_output_format(format, cx);
            }))
    }

    fn render_field(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let field = &self.fields[index];
        let mut row = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_end()
            .gap_2()
            .w_full()
            .child(div().w_40().child(LabeledField::new(
                "Name",
                field.name.render("sample-data.field.name"),
            )))
            .child(
                Button::new(format!("Type: {}", field.field_type.label()))
                    .id(format!("sample-data.field.{index}.type"))
                    .focus_handle(field.focus.cycle_type.clone())
                    .on_click(view_click(cx, move |this, _window, cx| {
                        this.cycle_field_type(index, cx);
                    })),
            );

        match field.field_type {
            SampleFieldType::Number => {
                row = row
                    .child(div().w_24().child(LabeledField::new(
                        "Min",
                        field.number_minimum.render("sample-data.field.number-min"),
                    )))
                    .child(div().w_24().child(LabeledField::new(
                        "Max",
                        field.number_maximum.render("sample-data.field.number-max"),
                    )))
                    .child(
                        Button::new(if field.number_is_integer {
                            "Integer: on"
                        } else {
                            "Integer: off"
                        })
                        .id(format!("sample-data.field.{index}.integer"))
                        .variant(if field.number_is_integer {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Secondary
                        })
                        .focus_handle(field.focus.toggle_integer.clone())
                        .on_click(view_click(
                            cx,
                            move |this, _window, cx| {
                                this.toggle_integer(index, cx);
                            },
                        )),
                    );
            }
            SampleFieldType::Date => {
                row = row
                    .child(div().w_24().child(LabeledField::new(
                        "From (s)",
                        field.date_minimum.render("sample-data.field.date-min"),
                    )))
                    .child(div().w_24().child(LabeledField::new(
                        "To (s)",
                        field.date_maximum.render("sample-data.field.date-max"),
                    )));
            }
            SampleFieldType::Enumeration => {
                row = row.child(div().w_64().child(LabeledField::new(
                    "Choices (comma-separated)",
                    field.choices.render("sample-data.field.choices"),
                )));
            }
            SampleFieldType::FictionalName
            | SampleFieldType::FictionalEmail
            | SampleFieldType::Boolean
            | SampleFieldType::Uuid => {}
        }

        row.child(
            Button::new("Up")
                .id(format!("sample-data.field.{index}.up"))
                .disabled(index == 0)
                .focus_handle(field.focus.up.clone())
                .on_click(view_click(cx, move |this, _window, cx| {
                    this.move_field(index, -1, cx);
                })),
        )
        .child(
            Button::new("Down")
                .id(format!("sample-data.field.{index}.down"))
                .disabled(index + 1 == self.fields.len())
                .focus_handle(field.focus.down.clone())
                .on_click(view_click(cx, move |this, _window, cx| {
                    this.move_field(index, 1, cx);
                })),
        )
        .child(
            Button::new("Remove")
                .id(format!("sample-data.field.{index}.remove"))
                .focus_handle(field.focus.remove.clone())
                .on_click(view_click(cx, move |this, _window, cx| {
                    this.remove_field(index, cx);
                })),
        )
    }

    fn render_fields(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .id("sample-data.fields")
            .flex()
            .flex_col()
            .gap_2()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        if self.fields.is_empty() {
            list = list.child(div().text_sm().child("Add a field to begin."));
        }
        for index in 0..self.fields.len() {
            list = list.child(self.render_field(index, cx));
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
        let history = HistoryPanel::new(
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
            .child(history)
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
                div().text_xs().text_color(tokens.text()).child(
                    "Restoring this History entry replaces the current Sample Data session.",
                ),
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

impl Render for SampleDataWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
        let can_copy = self.session.evaluation().is_valid_operation();

        let toolbar = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(self.format_button(SampleDataFormat::Json, cx))
            .child(self.format_button(SampleDataFormat::Csv, cx))
            .child(
                Button::new("−")
                    .disabled(self.row_count <= MINIMUM_ROW_COUNT)
                    .focus_handle(self.focus.row_decrement.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.adjust_row_count(-1, cx);
                    })),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(tokens.text())
                    .child(format!("Rows: {}", self.row_count)),
            )
            .child(
                Button::new("+")
                    .disabled(self.row_count >= MAXIMUM_ROW_COUNT)
                    .focus_handle(self.focus.row_increment.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.adjust_row_count(1, cx);
                    })),
            )
            .child(
                Button::new("Add Field")
                    .disabled(self.fields.len() >= MAXIMUM_FIELD_COUNT)
                    .focus_handle(self.focus.add_field.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.add_field(window, cx);
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
                Button::new("Generate")
                    .variant(ButtonVariant::Primary)
                    .focus_handle(self.focus.generate.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.generate(window, cx);
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

        let result_body = if matches!(self.session.evaluation(), SampleDataEvaluation::Empty) {
            empty_state("Choose fields and generate fictional sample rows").into_any_element()
        } else {
            self.result
                .render(true, "sample-data.result")
                .into_any_element()
        };

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
                            .child("Sample Data"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Fictional JSON/CSV rows, local and offline"),
                    ),
            )
            .child(div().text_xs().text_color(tokens.text_muted()).child(
                "Every generated identity is fictional. This local generator does not simulate locales, relationships, or real people.",
            ))
            .child(toolbar);

        if self.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("History is paused for Sample Data: {error}"),
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
                "Fields",
                "ordered schema, up to 50",
                self.render_fields(cx),
            ))
            .child(panel(
                format!("Generated {}", self.output_format.label()),
                "read-only, selectable",
                result_body,
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics())
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<SampleDataSnapshot> {
    if entry.snapshot_version != SampleData::SNAPSHOT_VERSION {
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

fn parse_number(text: &str) -> f64 {
    text.trim().parse::<f64>().unwrap_or(f64::NAN)
}

fn number_text(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 9.0e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RestoreDecision {
    Apply,
    Confirm,
}

/// Require confirmation when a nonempty session has different settings or
/// output. Comparing output catches generated batches whose schemas match.
fn restore_decision<C: PartialEq, O: PartialEq>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;

    use crate::history::{HistoryClock, HistoryEntry, HistoryStore};

    struct TestClipboard;

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            None
        }

        fn write_text(&self, _text: &str, _cx: &mut App) {}
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
            "sofdevtool-sample-data-restore-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn history_entry(id: &str, snapshot: SampleDataSnapshot) -> HistoryEntry {
        HistoryEntry {
            id: id.to_owned(),
            captured_at: "2026-01-01T00:00:00Z".to_owned(),
            utility_id: SampleData::ID.to_owned(),
            snapshot_version: SampleData::SNAPSHOT_VERSION,
            payload: serde_json::to_value(snapshot).unwrap(),
        }
    }

    #[test]
    fn preview_line_summarizes_without_losing_the_start() {
        assert_eq!(preview_line(""), "Empty result");
        assert_eq!(preview_line("[1, 2]\n"), "[1, 2]");
        let long = "x".repeat(120);
        assert_eq!(preview_line(&long).chars().count(), 81);
    }

    #[test]
    fn number_text_round_trips_the_bounded_defaults() {
        assert_eq!(number_text(0.0), "0");
        assert_eq!(number_text(100.0), "100");
        assert_eq!(number_text(1_893_456_000.0), "1893456000");
        assert_eq!(parse_number("0"), 0.0);
        assert!(parse_number("nope").is_nan());
    }

    #[test]
    fn different_generated_batches_with_identical_schema_require_confirmation() {
        let configuration = SampleDataRequest::default().configuration();
        assert_eq!(
            restore_decision(
                true,
                &configuration,
                &configuration,
                &Some("current batch"),
                &Some("captured batch"),
            ),
            RestoreDecision::Confirm,
        );
    }

    #[test]
    fn edited_invalid_schema_is_nonempty_and_protected() {
        use sofdevtool_core::utility::Utility;

        let mut fields = SampleDataRequest::default().fields;
        fields[0].name.clear();
        let edited = SampleDataRequest {
            fields,
            ..SampleDataRequest::default()
        };
        let evaluation = SampleData::evaluate(&edited);
        assert!(!evaluation.is_valid_operation());

        let captured = SampleDataRequest::default().configuration();
        let current = edited.configuration();
        assert_eq!(
            restore_decision(
                current != SampleDataRequest::default().configuration(),
                &current,
                &captured,
                &None::<&str>,
                &Some("captured output"),
            ),
            RestoreDecision::Confirm,
        );
    }

    #[test]
    fn empty_and_equivalent_sample_data_sessions_apply_without_confirmation() {
        let configuration = SampleDataRequest::default().configuration();
        assert_eq!(
            restore_decision(
                false,
                &configuration,
                &configuration,
                &None::<&str>,
                &Some("captured output"),
            ),
            RestoreDecision::Apply,
        );
        assert_eq!(
            restore_decision(
                true,
                &configuration,
                &configuration,
                &Some("same output"),
                &Some("same output"),
            ),
            RestoreDecision::Apply,
        );
    }

    #[gpui::test]
    fn workspace_restore_confirmation_cancel_and_confirm_use_captured_output(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(sofdevtool_ui::init);
        let root = isolated_root();
        let clock_calls = Rc::new(Cell::new(0));
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(CountingClock(clock_calls.clone())),
        ));
        let snapshot = SampleDataSnapshot {
            request: SampleDataRequest {
                generation: 7,
                ..SampleDataRequest::default()
            },
            rows: Vec::new(),
            output: "exact captured sample output".to_owned(),
        };
        let entry = history_entry("captured", snapshot.clone());
        history.store().record(entry.clone()).unwrap();
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard);
        let (workspace, cx) = cx.add_window_view(|window, cx| {
            SampleDataWorkspace::new(window, cx, clipboard, history.clone())
        });

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                let request = SampleDataRequest {
                    generation: 8,
                    ..SampleDataRequest::default()
                };
                let SubmitOutcome::Scheduled(revision) = view.session.submit(request.clone())
                else {
                    panic!("initial current batch must schedule");
                };
                assert!(view
                    .session
                    .publish(
                        revision,
                        SampleDataEvaluation::Valid {
                            output: "current generated output".to_owned(),
                            rows: Vec::new(),
                        },
                    )
                    .is_some());
                let current_evaluation = view.session.evaluation().clone();
                let current_request = view.session.request().unwrap().clone();

                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.pending_restore, Some(entry.clone()));
                view.cancel_restore(cx);
                assert!(view.pending_restore.is_none());
                assert_eq!(view.session.request(), Some(&current_request));
                assert_eq!(view.session.evaluation(), &current_evaluation);

                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.pending_restore, Some(entry.clone()));
                view.confirm_restore(window, cx);
                assert!(view.pending_restore.is_none());
                assert_eq!(
                    view.session.evaluation().output(),
                    Some(snapshot.output.as_str())
                );
                assert_eq!(
                    view.request(cx).configuration(),
                    snapshot.request.configuration()
                );
                assert_eq!(view.generation, 7);
                assert!(view.session.take_snapshot().is_none());
            });
            assert_eq!(history.load(SampleData::ID).unwrap().len(), 1);
            assert_eq!(clock_calls.get(), 0);
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn invalid_edited_sample_schema_requires_confirmation_and_survives_cancel(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(sofdevtool_ui::init);
        let root = isolated_root();
        let clock_calls = Rc::new(Cell::new(0));
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(CountingClock(clock_calls.clone())),
        ));
        let snapshot = SampleDataSnapshot {
            request: SampleDataRequest::default(),
            rows: Vec::new(),
            output: "captured valid result".to_owned(),
        };
        let entry = history_entry("captured", snapshot.clone());
        history.store().record(entry.clone()).unwrap();
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard);
        let (workspace, cx) = cx.add_window_view(|window, cx| {
            SampleDataWorkspace::new(window, cx, clipboard, history.clone())
        });

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.fields[0].name.assign_text("", window, cx);
                let invalid_request = view.request(cx);
                assert!(!SampleData::evaluate(&invalid_request).is_valid_operation());
                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.pending_restore, Some(entry.clone()));
                view.cancel_restore(cx);
                assert!(view.pending_restore.is_none());
                assert_eq!(view.fields[0].name.text(cx), "");
                assert!(!view.session.evaluation().is_valid_operation());

                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.pending_restore, Some(entry.clone()));
                view.confirm_restore(window, cx);
                assert!(view.pending_restore.is_none());
                assert_eq!(view.fields[0].name.text(cx), "name");
                assert_eq!(
                    view.session.evaluation().output(),
                    Some(snapshot.output.as_str())
                );
                assert_eq!(
                    view.request(cx).configuration(),
                    snapshot.request.configuration()
                );
                assert!(view.session.take_snapshot().is_none());
            });
            assert_eq!(history.load(SampleData::ID).unwrap().len(), 1);
            assert_eq!(clock_calls.get(), 0);
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn empty_and_equivalent_sample_data_workspaces_restore_without_confirmation(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(sofdevtool_ui::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(CountingClock(Rc::new(Cell::new(0)))),
        ));
        let snapshot = SampleDataSnapshot {
            request: SampleDataRequest::default(),
            rows: Vec::new(),
            output: "captured sample output".to_owned(),
        };
        let entry = history_entry("captured", snapshot.clone());
        history.store().record(entry.clone()).unwrap();
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard);
        let (workspace, cx) = cx.add_window_view(|window, cx| {
            SampleDataWorkspace::new(window, cx, clipboard, history.clone())
        });

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.request_restore(entry.clone(), window, cx);
                assert!(
                    view.pending_restore.is_none(),
                    "empty sessions apply directly"
                );
                assert_eq!(
                    view.session.evaluation().output(),
                    Some(snapshot.output.as_str())
                );
                view.request_restore(entry.clone(), window, cx);
                assert!(
                    view.pending_restore.is_none(),
                    "equivalent sessions apply directly"
                );
            });
            assert_eq!(history.load(SampleData::ID).unwrap().len(), 1);
        });
        fs::remove_dir_all(root).unwrap();
    }
}
