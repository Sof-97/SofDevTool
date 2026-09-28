//! The Sample Data workspace: edit a reordered, typed field list, choose JSON
//! or CSV and generate fictional rows on an explicit action. Output is
//! read-only; restoring a History entry rebuilds the exact schema and output
//! without regenerating.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, App, Context, Entity, IntoElement, Render, Subscription, Window};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState, NumberInput, TextareaState},
    list::ListState,
    tab::{Tab, TabBar},
    ActiveTheme as _, Disableable as _, Icon,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::sample_data::{
    FieldDefinition, SampleData, SampleDataEvaluation, SampleDataFormat, SampleDataRequest,
    SampleDataSnapshot, SampleFieldType, MAXIMUM_FIELD_COUNT, MAXIMUM_ROW_COUNT, MINIMUM_ROW_COUNT,
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

type SampleDataSession = Session<SampleData>;

/// Builds the Sample Data workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    layout: Entity<WorkspaceLayout>,
) -> WorkspaceViews {
    let workspace =
        cx.new(|cx| SampleDataWorkspace::new_with_layout(window, cx, clipboard, history, layout));
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

/// The editable state of one field, including its own text inputs so reordering
/// moves the whole editor and removal drops its subscriptions.
struct FieldState {
    ui_id: u64,
    field_type: SampleFieldType,
    number_is_integer: bool,
    name: Entity<InputState>,
    number_minimum: Entity<InputState>,
    number_maximum: Entity<InputState>,
    date_minimum: Entity<InputState>,
    date_maximum: Entity<InputState>,
    choices: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl FieldState {
    fn new(
        window: &mut Window,
        cx: &mut Context<SampleDataWorkspace>,
        definition: &FieldDefinition,
        ui_id: u64,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).default_value(definition.name.clone()));
        let number_minimum = cx.new(|cx| {
            InputState::new(window, cx).default_value(number_text(definition.number_minimum))
        });
        let number_maximum = cx.new(|cx| {
            InputState::new(window, cx).default_value(number_text(definition.number_maximum))
        });
        let date_minimum = cx.new(|cx| {
            InputState::new(window, cx).default_value(number_text(definition.date_minimum_seconds))
        });
        let date_maximum = cx.new(|cx| {
            InputState::new(window, cx).default_value(number_text(definition.date_maximum_seconds))
        });
        let choices = cx
            .new(|cx| InputState::new(window, cx).default_value(definition.enum_choices.join(",")));

        let mut subscriptions = Vec::new();
        for state in [
            &name,
            &number_minimum,
            &number_maximum,
            &date_minimum,
            &date_maximum,
            &choices,
        ] {
            subscriptions.push(cx.subscribe_in(
                state,
                window,
                |this, _entity, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.invalidate(cx);
                    }
                },
            ));
        }

        Self {
            ui_id,
            field_type: definition.field_type,
            number_is_integer: definition.number_is_integer,
            name,
            number_minimum,
            number_maximum,
            date_minimum,
            date_maximum,
            choices,
            _subscriptions: subscriptions,
        }
    }

    fn definition(&self, cx: &App) -> FieldDefinition {
        FieldDefinition {
            name: self.name.read(cx).value().to_string(),
            field_type: self.field_type,
            number_minimum: parse_number(&self.number_minimum.read(cx).value()),
            number_maximum: parse_number(&self.number_maximum.read(cx).value()),
            number_is_integer: self.number_is_integer,
            date_minimum_seconds: parse_number(&self.date_minimum.read(cx).value()),
            date_maximum_seconds: parse_number(&self.date_maximum.read(cx).value()),
            enum_choices: self
                .choices
                .read(cx)
                .value()
                .split(',')
                .map(str::to_owned)
                .collect(),
        }
    }
}

pub struct SampleDataWorkspace {
    fields: Vec<FieldState>,
    next_field_id: u64,
    row_count: u32,
    row_count_input: Entity<InputState>,
    output_format: SampleDataFormat,
    show_fields: bool,
    session: SampleDataSession,
    generation: u64,
    display_epoch: u64,
    result: Entity<TextareaState>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    layout: Entity<WorkspaceLayout>,
    _layout_subscription: Subscription,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl SampleDataWorkspace {
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
        let default = SampleDataRequest::default();
        let fields: Vec<FieldState> = default
            .fields
            .iter()
            .enumerate()
            .map(|(index, definition)| FieldState::new(window, cx, definition, index as u64))
            .collect();
        let next_field_id = fields.len() as u64;
        let result = cx.new(|cx| TextareaState::new(window, cx));
        let row_count_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(default.row_count.to_string())
                .min(f64::from(MINIMUM_ROW_COUNT))
                .max(f64::from(MAXIMUM_ROW_COUNT))
                .step(1_f64)
        });
        let row_count_subscription = cx.subscribe_in(
            &row_count_input,
            window,
            |this, _entity, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.commit_row_count(cx);
                }
            },
        );
        let history_view = HistoryViewState::load(&history, SampleData::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(SampleData::ID, move |cx| {
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
            fields,
            next_field_id,
            row_count: default.row_count,
            row_count_input,
            output_format: default.output,
            show_fields: true,
            session: SampleDataSession::new(),
            generation: 0,
            display_epoch: u64::MAX,
            result,
            clipboard,
            history,
            copied: false,
            suppress_changes: false,
            history_view,
            history_list,
            layout,
            _layout_subscription: layout_subscription,
            _history_subscription: history_subscription,
            _subscriptions: vec![row_count_subscription],
        };
        workspace.sync_history(cx);
        workspace
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
        self.show_fields = true;
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
        if self.session.evaluation().is_valid_operation() {
            self.show_fields = false;
        }
        self.record_settled(cx);
        self.sync_display(window, cx);
        cx.notify();
    }

    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a Sample Data snapshot serializes");
        let result = self
            .history
            .record(SampleData::ID, SampleData::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, SampleData::ID, result);
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, SampleData::ID);
        self.sync_history(cx);
        cx.notify();
    }

    /// Reflects the owning view's History rows and selection into the kit list.
    fn sync_history(&self, cx: &mut Context<Self>) {
        ui::history_set_rows(&self.history_list, self.history_items(), cx);
        ui::history_set_selected(&self.history_list, self.history_view.selected.clone(), cx);
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
        self.result
            .update(cx, |state, cx| state.set_value(output, window, cx));
    }

    fn set_output_format(&mut self, format: SampleDataFormat, cx: &mut Context<Self>) {
        if self.output_format != format {
            self.output_format = format;
            self.invalidate(cx);
        }
    }

    /// Clamps the typed/number-stepped row count back into range. The retained
    /// number input is reflected during render.
    fn commit_row_count(&mut self, cx: &mut Context<Self>) {
        let raw = self.row_count_input.read(cx).value().to_string();
        let Ok(parsed) = raw.trim().parse::<i64>() else {
            return;
        };
        let next = parsed.clamp(i64::from(MINIMUM_ROW_COUNT), i64::from(MAXIMUM_ROW_COUNT)) as u32;
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
        let ui_id = self.next_field_id;
        self.next_field_id = self.next_field_id.wrapping_add(1);
        self.fields
            .push(FieldState::new(window, cx, &definition, ui_id));
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

    fn set_field_type(
        &mut self,
        index: usize,
        field_type: SampleFieldType,
        cx: &mut Context<Self>,
    ) {
        if let Some(field) = self.fields.get_mut(index) {
            if field.field_type == field_type {
                return;
            }
            field.field_type = field_type;
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
        self.show_fields = true;
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
            self.history_view.error =
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
            self.history_view.pending_restore = Some(entry);
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current Sample Data session.",
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
        snapshot: SampleDataSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        let request = snapshot.request.clone();
        let mut next_field_id = self.next_field_id;
        self.fields = request
            .fields
            .iter()
            .map(|definition| {
                let ui_id = next_field_id;
                next_field_id = next_field_id.wrapping_add(1);
                FieldState::new(window, cx, definition, ui_id)
            })
            .collect();
        self.next_field_id = next_field_id;
        self.row_count = request.row_count;
        self.output_format = request.output;
        self.show_fields = false;
        self.session.restore(snapshot);
        self.generation = self.generation.max(
            self.session
                .request()
                .map_or(0, |request| request.generation),
        );
        self.suppress_changes = false;
        self.history_view.pending_restore = None;
        self.copied = false;
        self.display_epoch = u64::MAX;
        self.sync_history(cx);
        self.sync_display(window, cx);
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.history_view.pending_restore.clone() {
            if self
                .history_view
                .retained(&self.history, SampleData::ID, &entry)
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
                .retained(&self.history, SampleData::ID, &entry)
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
                        .map(|snapshot| preview_line(&snapshot.output))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: snapshot.is_none().then(|| "Unavailable".to_owned()),
                    selectable: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn render_field(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let field = &self.fields[index];
        let field_id = field.ui_id;
        let selected_type = SampleFieldType::ALL
            .iter()
            .position(|field_type| *field_type == field.field_type)
            .unwrap_or(0);
        let field_type_control = TabBar::new(format!("sample-data.field.{field_id}.type"))
            .segmented()
            .selected_index(selected_type)
            .children(
                SampleFieldType::ALL
                    .into_iter()
                    .map(|field_type| Tab::new().label(field_type.label())),
            )
            .on_click(cx.listener(move |this, choice, _window, cx| {
                if let Some(field_type) = SampleFieldType::ALL.get(*choice) {
                    this.set_field_type(index, *field_type, cx);
                }
            }));
        let mut row = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_end()
            .gap_3()
            .w_full()
            .child(
                div().w_40().child(ui::labeled_field(
                    cx,
                    "Name",
                    None::<String>,
                    Input::new(&field.name)
                        .accessibility_id("sample-data.field.name")
                        .w_full(),
                )),
            )
            .child(field_type_control);

        match field.field_type {
            SampleFieldType::Number => {
                row = row
                    .child(
                        div().w_24().child(ui::labeled_field(
                            cx,
                            "Min",
                            None::<String>,
                            Input::new(&field.number_minimum)
                                .accessibility_id("sample-data.field.number-min")
                                .w_full(),
                        )),
                    )
                    .child(
                        div().w_24().child(ui::labeled_field(
                            cx,
                            "Max",
                            None::<String>,
                            Input::new(&field.number_maximum)
                                .accessibility_id("sample-data.field.number-max")
                                .w_full(),
                        )),
                    )
                    .child(
                        Button::new(format!("sample-data.field.{field_id}.integer"))
                            .label(if field.number_is_integer {
                                "Integer: on"
                            } else {
                                "Integer: off"
                            })
                            .when(field.number_is_integer, |button| button.primary())
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.toggle_integer(index, cx);
                            })),
                    );
            }
            SampleFieldType::Date => {
                row = row
                    .child(
                        div().w_24().child(ui::labeled_field(
                            cx,
                            "From (s)",
                            None::<String>,
                            Input::new(&field.date_minimum)
                                .accessibility_id("sample-data.field.date-min")
                                .w_full(),
                        )),
                    )
                    .child(
                        div().w_24().child(ui::labeled_field(
                            cx,
                            "To (s)",
                            None::<String>,
                            Input::new(&field.date_maximum)
                                .accessibility_id("sample-data.field.date-max")
                                .w_full(),
                        )),
                    );
            }
            SampleFieldType::Enumeration => {
                row = row.child(
                    div().w_64().child(ui::labeled_field(
                        cx,
                        "Choices (comma-separated)",
                        None::<String>,
                        Input::new(&field.choices)
                            .accessibility_id("sample-data.field.choices")
                            .w_full(),
                    )),
                );
            }
            SampleFieldType::FictionalName
            | SampleFieldType::FictionalEmail
            | SampleFieldType::Boolean
            | SampleFieldType::Uuid => {}
        }

        row.child(
            Button::new(format!("sample-data.field.{field_id}.up"))
                .label("Up")
                .disabled(index == 0)
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.move_field(index, -1, cx);
                })),
        )
        .child(
            Button::new(format!("sample-data.field.{field_id}.down"))
                .label("Down")
                .disabled(index + 1 == self.fields.len())
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.move_field(index, 1, cx);
                })),
        )
        .child(
            Button::new(format!("sample-data.field.{field_id}.remove"))
                .label("Remove")
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.remove_field(index, cx);
                })),
        )
    }

    fn render_fields(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .id("sample-data.fields")
            .flex()
            .flex_col()
            .gap_4()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_4();
        if self.fields.is_empty() {
            list = list.child(div().text_sm().child("Add a field to begin."));
        }
        for index in 0..self.fields.len() {
            list = list.child(self.render_field(index, cx));
        }
        list
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
        let restore = Button::new("sample-data.history.restore-selected")
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

impl Render for SampleDataWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let theme = cx.theme().clone();
        let can_copy = self.session.evaluation().is_valid_operation();
        let compact = f32::from(window.bounds().size.width) < 1500.;

        // Keep the retained row-count input showing the clamped value.
        // Programmatic `set_value` emits no change, so this cannot loop.
        let row_count_text = self.row_count.to_string();
        self.row_count_input.update(cx, |state, cx| {
            if state.value().as_ref() != row_count_text.as_str() {
                state.set_value(row_count_text.clone(), window, cx);
            }
        });

        let selected_format = SampleDataFormat::ALL
            .iter()
            .position(|format| *format == self.output_format)
            .unwrap_or(0);
        let output_format = TabBar::new("sample-data.output-format")
            .segmented()
            .selected_index(selected_format)
            .children(
                SampleDataFormat::ALL
                    .into_iter()
                    .map(|format| Tab::new().label(format.label())),
            )
            .on_click(cx.listener(|this, choice, _window, cx| {
                if let Some(format) = SampleDataFormat::ALL.get(*choice) {
                    this.set_output_format(*format, cx);
                }
            }));

        let row_count = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Rows"),
            )
            .child(div().w_32().child(NumberInput::new(&self.row_count_input)));

        let toolbar = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(output_format)
            .child(row_count)
            .child(
                Button::new("sample-data.add-field")
                    .label("Add Field")
                    .disabled(self.fields.len() >= MAXIMUM_FIELD_COUNT)
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.add_field(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(ui::copy_feedback(cx, self.copied, "Copied to Clipboard"))
            .child(
                Button::new("sample-data.generate")
                    .label("Generate")
                    .primary()
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.generate(window, cx);
                    })),
            )
            .child(
                Button::new("sample-data.copy-result")
                    .icon(Icon::new(IconName::Copy))
                    .tooltip("Copy generated result")
                    .accessibility_label("Copy generated result")
                    .disabled(!can_copy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::new("sample-data.clear")
                    .label("Clear")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.clear(window, cx);
                    })),
            );

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
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
                            .child("Sample Data"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Fictional JSON/CSV rows, local and offline"),
                    ),
            )
            .child(div().text_xs().text_color(theme.muted_foreground).child(
                "Every generated identity is fictional. This local generator does not simulate locales, relationships, or real people.",
            ))
            .child(toolbar);

        if compact {
            column = column.child(
                div().w(gpui::px(180.)).child(
                    TabBar::new("sample-data.view")
                        .segmented()
                        .selected_index(usize::from(!self.show_fields))
                        .children([Tab::new().label("Fields"), Tab::new().label("Result")])
                        .on_click(cx.listener(|this, choice, _window, cx| {
                            this.show_fields = *choice == 0;
                            cx.notify();
                        })),
                ),
            );
        }

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Sample Data History: {error}"),
                None,
            ));
        }

        let mut workspace = div().flex().flex_row().gap_4().flex_1().min_h_0().min_w_0();
        if !compact || self.show_fields {
            let fields_body = self.render_fields(cx);
            workspace = workspace.child(ui::panel(
                cx,
                "Fields",
                "ordered schema, up to 50",
                fields_body,
            ));
        }
        if !compact || !self.show_fields {
            let result_body = if matches!(self.session.evaluation(), SampleDataEvaluation::Empty) {
                ui::empty_state(cx, "Choose fields and generate fictional sample rows")
                    .into_any_element()
            } else {
                ui::multiline_editor(&self.result, true, "sample-data.result").into_any_element()
            };
            workspace = workspace.child(ui::panel(
                cx,
                format!("Generated {}", self.output_format.label()),
                "read-only, selectable",
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .child(result_body),
            ));
        }
        if self.layout.read(cx).placement(UtilityId::SampleData) == HistoryPlacement::Inline {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics(cx))
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
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use gpui::VisualTestContext;
    use gpui_kit::component::Root;

    use crate::history::{HistoryClock, HistoryEntry, HistoryStore};

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
            "sofdevtool-sample-data-restore-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
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
        cx.update(gpui_kit::init);
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
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| SampleDataWorkspace::new(window, cx, clipboard, history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

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
                assert_eq!(view.history_view.pending_restore, Some(entry.clone()));
                view.cancel_restore(cx);
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(view.session.request(), Some(&current_request));
                assert_eq!(view.session.evaluation(), &current_evaluation);

                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.history_view.pending_restore, Some(entry.clone()));
                view.confirm_restore(window, cx);
                assert!(view.history_view.pending_restore.is_none());
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
            workspace.update(cx, |view, _| {
                assert!(view.history_view.select(&entry.id));
                view.history_view.pending_restore = Some(entry.clone());
            });
            history.clear_utility(SampleData::ID, cx).unwrap();
            let view = workspace.read(cx);
            assert!(view.history_view.entries.is_empty());
            assert!(view.history_view.selected.is_none());
            assert!(view.history_view.pending_restore.is_none());
            assert_eq!(
                view.session.evaluation().output(),
                Some(snapshot.output.as_str())
            );
            assert_eq!(view.result.read(cx).value().to_string(), snapshot.output);
            workspace.update(cx, |view, cx| {
                view.history_view.pending_restore = Some(entry.clone());
                view.confirm_restore(window, cx);
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(
                    view.session.evaluation().output(),
                    Some(snapshot.output.as_str())
                );
            });
            assert_eq!(clock_calls.get(), 0);
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn invalid_edited_sample_schema_requires_confirmation_and_survives_cancel(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
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
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| SampleDataWorkspace::new(window, cx, clipboard, history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.fields[0]
                    .name
                    .update(cx, |state, cx| state.set_value("", window, cx));
                let invalid_request = view.request(cx);
                assert!(!SampleData::evaluate(&invalid_request).is_valid_operation());
                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.history_view.pending_restore, Some(entry.clone()));
                view.cancel_restore(cx);
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(view.fields[0].name.read(cx).value().to_string(), "");
                assert!(!view.session.evaluation().is_valid_operation());

                view.request_restore(entry.clone(), window, cx);
                assert_eq!(view.history_view.pending_restore, Some(entry.clone()));
                view.confirm_restore(window, cx);
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(view.fields[0].name.read(cx).value().to_string(), "name");
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
        cx.update(gpui_kit::init);
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
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| SampleDataWorkspace::new(window, cx, clipboard, history.clone()));
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
                assert_eq!(
                    view.session.evaluation().output(),
                    Some(snapshot.output.as_str())
                );
                view.request_restore(entry.clone(), window, cx);
                assert!(
                    view.history_view.pending_restore.is_none(),
                    "equivalent sessions apply directly"
                );
            });
            assert_eq!(history.load(SampleData::ID).unwrap().len(), 1);
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn compact_schema_result_transition_preserves_generated_copy(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(crate::history::SystemClock::new()),
        ));
        let clipboard = Rc::new(TestClipboard::default());
        let app_clipboard: Rc<dyn Clipboard> = clipboard.clone();
        let mut captured = None;
        let window = cx.open_window(gpui::size(gpui::px(1000.), gpui::px(700.)), |window, cx| {
            let view =
                cx.new(|cx| SampleDataWorkspace::new(window, cx, app_clipboard, history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        assert!(workspace.read_with(&cx, |view, _| view.show_fields));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.set_output_format(SampleDataFormat::Csv, cx);
                assert_eq!(view.output_format, SampleDataFormat::Csv);
                view.set_field_type(0, SampleFieldType::Number, cx);
                assert_eq!(view.fields[0].field_type, SampleFieldType::Number);
            });
            window.draw(cx).clear(cx);
        });

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.generate(window, cx);
                view.generate(window, cx);
            });
            assert!(!workspace.read(cx).show_fields);
            window.draw(cx).clear(cx);
        });
        let output = workspace.read_with(&cx, |view, _| {
            view.session.evaluation().output().unwrap().to_owned()
        });
        assert!(
            output.contains(','),
            "CSV output should be copied as generated"
        );
        assert_eq!(history.load(SampleData::ID).unwrap().len(), 2);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.copy_result(cx));
            workspace.update(cx, |view, cx| view.clear(window, cx));
            assert!(workspace.read(cx).show_fields);
            window.draw(cx).clear(cx);
        });
        assert_eq!(clipboard.0.borrow().as_deref(), Some(output.as_str()));
        assert_eq!(history.load(SampleData::ID).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
