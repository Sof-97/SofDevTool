//! The JSON Utility workspace: input, format/minify/query controls, diagnostics,
//! an explicit Clipboard surface and fresh Rust History.
//!
//! Input changes are debounced and revision-gated through the shared session, so
//! an obsolete asynchronous completion can never publish a result. One settled
//! valid operation is recorded once; preview and restore never reevaluate.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    anchored, deferred, div, point, px, App, Context, Entity, Focusable, IntoElement, MouseButton,
    Render, StatefulInteractiveElement, Subscription, Window,
};
use gpui_kit::assets::IconName;
use gpui_kit::base::input::{Enter, Escape, IndentInline, MoveDown, MoveUp};
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    input::{EditorState, Input, InputEvent, InputState},
    list::{ListItem, ListState},
    resizable::{h_resizable, resizable_panel, ResizableState},
    select::{Select, SelectEvent, SelectState},
    tab::{Tab, TabBar},
    ActiveTheme as _, Disableable as _, Icon, IndexPath, ThemeStyled as _,
};
use sofdevtool_core::json::{
    Indentation, Json, JsonCompletion, JsonEvaluation, JsonMode, JsonQueryIndex, JsonRequest,
    JsonSession, JsonSnapshot, Severity,
};
use sofdevtool_core::session::SubmitOutcome;
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::registry::UtilityId;
use crate::ui;
use crate::workspace_layout::{HistoryPlacement, WorkspaceLayout};

const DEBOUNCE: Duration = Duration::from_millis(250);

pub struct JsonWorkspace {
    input: Entity<EditorState>,
    result: Entity<EditorState>,
    error_highlight: gpui_kit::base::input::TextDecorationCollection,
    query: Entity<InputState>,
    query_index: Option<JsonQueryIndex>,
    indexed_source: String,
    completion_selected: usize,
    completion_dismissed: Option<(String, usize)>,
    formatted_source: Option<String>,
    indentation_select: Entity<SelectState<Vec<&'static str>>>,
    editor_split: Entity<ResizableState>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    mode: JsonMode,
    indentation: Indentation,
    sort_keys: bool,
    session: JsonSession,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    layout: Entity<WorkspaceLayout>,
    _layout_subscription: Subscription,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl JsonWorkspace {
    #[cfg(test)]
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let layout = cx.new(|_| WorkspaceLayout::load(None));
        Self::new_with_layout(window, cx, clipboard, history, layout)
    }

    pub fn new_with_layout(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
        layout: Entity<WorkspaceLayout>,
    ) -> Self {
        let layout_subscription = cx.observe(&layout, |_, _, cx| cx.notify());
        let input = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .folding(false)
                .auto_close(false)
        });
        let error_highlight = input.update(cx, |state, cx| {
            state.create_decorations_collection(vec![], cx)
        });
        let result = cx.new(|cx| EditorState::new(window, cx).language("json"));
        let query = cx.new(|cx| InputState::new(window, cx));
        let indentation_select = cx.new(|cx| {
            SelectState::new(
                vec!["2 spaces", "4 spaces"],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let editor_split = cx.new(|_| ResizableState::default());

        let subscriptions = vec![
            cx.observe(&query, |_, _, cx| cx.notify()),
            cx.subscribe_in(
                &input,
                window,
                |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.schedule(window, cx);
                    }
                    cx.notify();
                },
            ),
            cx.subscribe_in(
                &query,
                window,
                |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.completion_selected = 0;
                        this.schedule(window, cx);
                    }
                    cx.notify();
                },
            ),
            cx.subscribe_in(
                &indentation_select,
                window,
                |this, _, event: &SelectEvent<Vec<&'static str>>, window, cx| {
                    let SelectEvent::Confirm(Some(value)) = event else {
                        return;
                    };
                    this.set_indentation(
                        if *value == "4 spaces" {
                            Indentation::FourSpaces
                        } else {
                            Indentation::TwoSpaces
                        },
                        window,
                        cx,
                    );
                },
            ),
        ];

        let history_view = HistoryViewState::load(&history, Json::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Json::ID, move |cx| {
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
            input,
            result,
            error_highlight,
            query,
            query_index: None,
            indexed_source: String::new(),
            completion_selected: 0,
            completion_dismissed: None,
            formatted_source: None,
            indentation_select,
            editor_split,
            clipboard,
            history,
            mode: JsonMode::Format,
            indentation: Indentation::TwoSpaces,
            sort_keys: false,
            session: JsonSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            layout,
            _layout_subscription: layout_subscription,
            history_list,
            _history_subscription: history_subscription,
            _subscriptions: subscriptions,
        };
        workspace.sync_history(cx);
        workspace
    }

    fn request(&self, cx: &App) -> JsonRequest {
        JsonRequest {
            input: self.input.read(cx).value().to_string(),
            mode: self.mode,
            indentation: self.indentation,
            sort_keys: self.sort_keys,
            query: self.query.read(cx).value().to_string(),
        }
    }

    /// Submits the current request. A changed request clears the visible result
    /// immediately and schedules a debounced, revision-gated evaluation.
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.suppress_changes {
            return;
        }
        let request = self.request(cx);
        self.refresh_query_index(&request.input);
        let SubmitOutcome::Scheduled(revision) = self.session.submit(request) else {
            return;
        };
        self.sync_display(window, cx);
        cx.notify();

        let executor = cx.background_executor().clone();
        cx.spawn_in(window, async move |this, cx| {
            executor.timer(DEBOUNCE).await;
            cx.update(|window, cx| {
                this.update(cx, |this, cx| {
                    if this.session.revision() != revision {
                        return;
                    }
                    this.format_query_input(window, cx);
                    let request = this.request(cx);
                    this.refresh_query_index(&request.input);
                    this.session.submit(request);
                    this.session.resolve(this.session.revision());
                    this.record_settled(cx);
                    cx.notify();
                })
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
        let payload = serde_json::to_value(&snapshot).expect("a JSON snapshot serializes");
        let result = self
            .history
            .record(Json::ID, Json::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Json::ID, result);
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Json::ID);
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
        let value = match self.session.evaluation() {
            JsonEvaluation::Valid { output } => output.clone(),
            _ => String::new(),
        };
        self.result
            .update(cx, |state, cx| state.set_value(value, window, cx));
        let text = self.input.read(cx).value().to_string();
        let color = cx.theme().danger.opacity(0.18);
        let decorations = self
            .session
            .evaluation()
            .diagnostics()
            .iter()
            .filter_map(|diagnostic| diagnostic.location)
            .filter_map(|location| error_line_range(&text, location.line))
            .map(|range| {
                gpui_kit::base::input::TextDecoration::new(
                    range,
                    gpui::HighlightStyle {
                        background_color: Some(color),
                        ..Default::default()
                    },
                )
            })
            .collect();
        self.error_highlight.set(decorations, cx);
        self.input.update(cx, |state, cx| {
            if let Some(markers) = state.diagnostics_mut() {
                markers.clear();
                for diagnostic in self.session.evaluation().diagnostics() {
                    let Some(location) = diagnostic.location else {
                        continue;
                    };
                    let Some(range) = error_line_range(&text, location.line) else {
                        continue;
                    };
                    let row = location.line - 1;
                    let end = text[range].encode_utf16().count() as u32;
                    markers.push(
                        gpui_kit::base::input::Diagnostic::new(
                            gpui_kit::base::input::Position::new(row, 0)
                                ..gpui_kit::base::input::Position::new(row, end),
                            diagnostic.message.clone(),
                        )
                        .with_severity(gpui_kit::base::input::DiagnosticSeverity::Error),
                    );
                }
            }
            cx.notify();
        });
    }

    fn set_mode(&mut self, mode: JsonMode, window: &mut Window, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.completion_dismissed = None;
        if mode == JsonMode::Query {
            self.formatted_source = None;
            self.format_query_input(window, cx);
            self.query.update(cx, |state, cx| state.focus(window, cx));
        }
        self.schedule(window, cx);
    }

    fn refresh_query_index(&mut self, source: &str) {
        if self.indexed_source != source {
            self.indexed_source = source.to_owned();
            self.query_index = JsonQueryIndex::new(source);
            self.completion_selected = 0;
            self.completion_dismissed = None;
        }
    }

    fn format_query_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.input.read(cx).value().to_string();
        if self.mode != JsonMode::Query
            || source.contains('\n')
            || self.formatted_source.as_ref() == Some(&source)
        {
            return;
        }
        let mut request = self.request(cx);
        request.mode = JsonMode::Format;
        let JsonEvaluation::Valid { output } = Json::evaluate(&request) else {
            return;
        };
        if output == source {
            return;
        }
        self.formatted_source = Some(source);
        self.suppress_changes = true;
        self.input
            .update(cx, |state, cx| state.replace_all(output, window, cx));
        self.suppress_changes = false;
    }

    fn query_completions(&self, cx: &App) -> Vec<JsonCompletion> {
        let query = self.query.read(cx);
        if self.completion_dismissed.as_ref() == Some(&(query.value().to_string(), query.cursor()))
        {
            return vec![];
        }
        self.query_index
            .as_ref()
            .map(|index| index.completions(&query.value(), query.cursor()))
            .unwrap_or_default()
    }

    fn accept_completion(
        &mut self,
        completion: JsonCompletion,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.query.update(cx, |state, cx| {
            state.set_selected_range(completion.range, cx);
            state.replace(completion.text, window, cx);
            state.focus(window, cx);
        });
        self.schedule(window, cx);
        self.dismiss_completions(cx);
        cx.notify();
    }

    fn dismiss_completions(&mut self, cx: &App) {
        let query = self.query.read(cx);
        self.completion_dismissed = Some((query.value().to_string(), query.cursor()));
    }

    fn query_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.query.read(cx).focus_handle(cx).is_focused(window) {
            cx.propagate();
            return;
        }
        let completions = self.query_completions(cx);
        if completions.is_empty() {
            cx.propagate();
            return;
        }
        let selected = self.completion_selected.min(completions.len() - 1);
        match key {
            "down" => self.completion_selected = (selected + 1) % completions.len(),
            "up" => {
                self.completion_selected = (selected + completions.len() - 1) % completions.len()
            }
            "tab" | "enter" => self.accept_completion(completions[selected].clone(), window, cx),
            "escape" => self.dismiss_completions(cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn set_indentation(
        &mut self,
        indentation: Indentation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.indentation == indentation {
            return;
        }
        self.indentation = indentation;
        self.schedule(window, cx);
    }

    fn set_sort(&mut self, sort_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.sort_keys == sort_keys {
            return;
        }
        self.sort_keys = sort_keys;
        self.schedule(window, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.input
                .update(cx, |state, cx| state.replace_all(text, window, cx));
        }
    }

    fn copy_result(&mut self, cx: &mut Context<Self>) {
        if let JsonEvaluation::Valid { output } = self.session.evaluation() {
            let output = output.clone();
            self.clipboard.write_text(&output, cx);
            self.copied = true;
            cx.notify();
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = false;
        self.input
            .update(cx, |state, cx| state.replace_all("", window, cx));
    }

    /// Requests a restore. Replacing a different nonempty session is confirmed
    /// first; otherwise the snapshot applies immediately.
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
        let current = self.input.read(cx).value().to_string();
        if !current.trim().is_empty() && current != snapshot.request.input {
            self.history_view.pending_restore = Some(entry);
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current non-empty JSON session.",
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

    /// Applies a captured snapshot directly. It never reevaluates input, never
    /// records, and invalidates any pending completion from the prior revision.
    fn apply_restore(
        &mut self,
        snapshot: JsonSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.mode = snapshot.request.mode;
        self.indentation = snapshot.request.indentation;
        self.indentation_select.update(cx, |state, cx| {
            state.set_selected_value(
                &match self.indentation {
                    Indentation::TwoSpaces => "2 spaces",
                    Indentation::FourSpaces => "4 spaces",
                },
                window,
                cx,
            )
        });
        self.sort_keys = snapshot.request.sort_keys;
        self.input.update(cx, |state, cx| {
            state.set_value(snapshot.request.input.clone(), window, cx)
        });
        self.query.update(cx, |state, cx| {
            state.set_value(snapshot.request.query.clone(), window, cx)
        });
        self.session.restore(snapshot);
        let source = self.input.read(cx).value().to_string();
        self.refresh_query_index(&source);
        self.formatted_source = None;
        self.dismiss_completions(cx);
        self.display_epoch = u64::MAX;
        self.suppress_changes = false;
        self.history_view.pending_restore = None;
        self.copied = false;
        self.sync_history(cx);
        self.sync_display(window, cx);
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.history_view.pending_restore.clone() {
            let retained = self.history_view.retained(&self.history, Json::ID, &entry);
            if retained {
                let Some(snapshot) = decode_snapshot(&entry) else {
                    return;
                };
                self.apply_restore(snapshot, window, cx);
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
            if self.history_view.retained(&self.history, Json::ID, &entry) {
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
                    status: snapshot
                        .is_none()
                        .then(|| "Unavailable snapshot".to_owned()),
                    selectable: true,
                }
            })
            .collect()
    }

    fn render_diagnostics(&self, cx: &App) -> impl IntoElement {
        let mut column = div().flex().flex_col().gap_2().w_full();
        for diagnostic in self.session.evaluation().diagnostics() {
            let severity = match diagnostic.severity {
                Severity::Error => ui::DiagnosticSeverity::Error,
                Severity::Warning => ui::DiagnosticSeverity::Warning,
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

    pub(crate) fn render_history(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.history_view.selected.clone();
        let restore_enabled = selected
            .as_ref()
            .and_then(|id| {
                self.history_view
                    .entries
                    .iter()
                    .find(|entry| &entry.id == id)
            })
            .is_some_and(|entry| decode_snapshot(entry).is_some());

        let restore = Button::new("json.history.restore-selected")
            .label("Restore selected")
            .disabled(!restore_enabled)
            .on_click(cx.listener(|this, _event, window, cx| this.restore_selected(window, cx)));

        let theme = cx.theme();
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
                format!("{}/25", self.history_view.entries.len()),
                restore,
            ))
    }
}

impl Render for JsonWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);

        let theme = cx.theme().clone();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), JsonEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.trim().is_empty())
                .unwrap_or(false);

        let mode_index = match self.mode {
            JsonMode::Format => 0,
            JsonMode::Minify => 1,
            JsonMode::Query => 2,
        };
        let mode_control = TabBar::new("json.mode")
            .segmented()
            .selected_index(mode_index)
            .children([
                Tab::new().label("Format"),
                Tab::new().label("Minify"),
                Tab::new().label("Query"),
            ])
            .on_click(cx.listener(|this, index, window, cx| {
                let mode = match index {
                    0 => JsonMode::Format,
                    1 => JsonMode::Minify,
                    _ => JsonMode::Query,
                };
                this.set_mode(mode, window, cx);
            }));

        let sort_owner = cx.weak_entity();
        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(mode_control)
            .when(self.mode == JsonMode::Format, |toolbar| {
                toolbar.child(
                    div().w(px(132.)).flex_shrink_0().child(
                        Select::new(&self.indentation_select)
                            .id("json.indentation")
                            .cleanable(false)
                            .accessibility_label("Indentation"),
                    ),
                )
            })
            .child(
                Checkbox::new("json.sort-keys")
                    .label("Sort keys")
                    .checked(self.sort_keys)
                    .on_change(move |checked, window, cx| {
                        sort_owner
                            .update(cx, |this, cx| this.set_sort(*checked, window, cx))
                            .ok();
                    }),
            )
            .child(div().flex_1());

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(theme.background)
            .text_color(theme.foreground)
            .gap_3()
            .child(toolbar);

        if self.mode == JsonMode::Query {
            let completions = if self.query.read(cx).focus_handle(cx).is_focused(window) {
                self.query_completions(cx)
            } else {
                vec![]
            };
            let selected = self
                .completion_selected
                .min(completions.len().saturating_sub(1));
            let suggestions = (!completions.is_empty()).then(|| {
                let query = self.query.read(cx);
                let bounds = query.input_bounds();
                let width = (window.bounds().size.width - px(16.)).min(px(380.));
                let origin = point(
                    query
                        .cursor_layout()
                        .map_or(bounds.origin.x + px(8.), |(caret, _)| caret.origin.x),
                    bounds.origin.y + bounds.size.height + px(4.),
                );
                let mut rows = div().flex().flex_col();
                for (index, completion) in completions.into_iter().enumerate() {
                    let owner = cx.weak_entity();
                    let hover_owner = owner.clone();
                    let label = completion.label.clone();
                    let kind = completion.kind.label();
                    let preview = completion.preview.clone();
                    rows = rows.child(
                        ListItem::new(("json.completion", index))
                            .role(gpui::accesskit::Role::ListBoxOption)
                            .accessibility_id(format!("json.completion.{index}"))
                            .aria_label(format!("Complete path {label}, {kind}, {preview}"))
                            .selected(index == selected)
                            .child(
                                div()
                                    .flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .child(div().flex_1().min_w_0().truncate().child(label.clone()))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .text_color(theme.muted_foreground)
                                            .child(kind)
                                            .child(
                                                div()
                                                    .max_w(px(170.))
                                                    .truncate()
                                                    .child(preview.clone()),
                                            ),
                                    ),
                            )
                            .on_mouse_enter(move |_, _, cx| {
                                owner
                                    .update(cx, |this, cx| {
                                        this.completion_selected = index;
                                        cx.notify();
                                    })
                                    .ok();
                            })
                            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .on_click(move |_, window, cx| {
                                hover_owner
                                    .update(cx, |this, cx| {
                                        this.accept_completion(completion.clone(), window, cx)
                                    })
                                    .ok();
                            }),
                    );
                }
                deferred(
                    anchored().position(origin).child(
                        div()
                            .id("json.completions")
                            .occlude()
                            .w(width)
                            .max_h(px(304.))
                            .overflow_y_scroll()
                            .popover_style(cx)
                            .child(rows)
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                this.dismiss_completions(cx);
                                cx.notify();
                            })),
                    ),
                )
            });
            column =
                column.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .capture_action(cx.listener(|this, _: &MoveDown, window, cx| {
                            this.query_key("down", window, cx)
                        }))
                        .capture_action(cx.listener(|this, _: &MoveUp, window, cx| {
                            this.query_key("up", window, cx)
                        }))
                        .capture_action(cx.listener(|this, _: &IndentInline, window, cx| {
                            this.query_key("tab", window, cx)
                        }))
                        .capture_action(cx.listener(|this, _: &Escape, window, cx| {
                            this.query_key("escape", window, cx)
                        }))
                        .capture_action(cx.listener(|this, event: &Enter, window, cx| {
                            if event.shift || event.secondary {
                                cx.propagate();
                            } else {
                                this.query_key("enter", window, cx);
                            }
                        }))
                        .child(
                            div().flex().items_center().gap_2().child("Path").child(
                                div().flex_1().min_w_0().child(
                                    Input::new(&self.query)
                                        .accessibility_id("json.query")
                                        .w_full(),
                                ),
                            ),
                        )
                        .when_some(suggestions, |this, suggestions| this.child(suggestions)),
                );
        }

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("JSON History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            ui::empty_state(cx, "Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), JsonEvaluation::Empty) {
            ui::empty_state(cx, "Paste JSON to begin").into_any_element()
        } else {
            ui::json_editor(&self.result, true, "json.result").into_any_element()
        };

        let input_actions = div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                Button::new("json.paste")
                    .icon(Icon::new(IconName::ClipboardPaste))
                    .tooltip("Paste into input")
                    .accessibility_label("Paste into input")
                    .ghost()
                    .on_click(cx.listener(|this, _event, window, cx| this.paste(window, cx))),
            )
            .child(
                Button::new("json.clear")
                    .icon(Icon::new(IconName::Trash))
                    .tooltip("Clear input")
                    .accessibility_label("Clear input")
                    .ghost()
                    .on_click(cx.listener(|this, _event, window, cx| this.clear(window, cx))),
            );
        let result_actions = div()
            .flex()
            .items_center()
            .gap_1()
            .child(ui::copy_feedback(cx, self.copied, "Copied"))
            .child(
                Button::new("json.copy-result")
                    .icon(Icon::new(IconName::Copy))
                    .tooltip("Copy result")
                    .accessibility_label("Copy result")
                    .ghost()
                    .disabled(!can_copy)
                    .on_click(cx.listener(|this, _event, _window, cx| this.copy_result(cx))),
            );
        let editors = h_resizable("json.editors")
            .with_state(&self.editor_split)
            .child(
                resizable_panel()
                    .size_range(px(220.)..px(2000.))
                    .child(ui::pane(
                        cx,
                        "Input",
                        input_actions,
                        ui::json_editor(&self.input, false, "json.input"),
                    )),
            )
            .child(
                resizable_panel()
                    .size_range(px(220.)..px(2000.))
                    .child(ui::pane(cx, "Result", result_actions, result_body)),
            );
        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(div().flex().flex_1().min_w_0().min_h_0().child(editors));
        if self.layout.read(cx).placement(UtilityId::Json) == HistoryPlacement::Inline {
            workspace = workspace.child(self.render_history(cx));
        }

        column.child(workspace).child(self.render_diagnostics(cx))
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<JsonSnapshot> {
    if entry.snapshot_version != Json::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

/// A short, single-line preview of a retained output.
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

fn error_line_range(text: &str, line: u32) -> Option<std::ops::Range<usize>> {
    let target = line.checked_sub(1)? as usize;
    let mut start = 0;
    for (index, content) in text.split('\n').enumerate() {
        if index == target {
            return Some(start..start + content.len());
        }
        start += content.len() + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use gpui::VisualTestContext;
    use gpui_kit::component::Root;

    use crate::history::{HistoryStore, SystemClock};

    struct TestClipboard;

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            None
        }

        fn write_text(&self, _text: &str, _cx: &mut App) {}
    }

    fn isolated_root() -> PathBuf {
        static NEXT_ROOT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "sofdevtool-json-list-{}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[gpui::test]
    fn compact_query_input_formats_completes_and_restores_without_recording(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| {
                JsonWorkspace::new(window, cx, Rc::new(TestClipboard), Rc::clone(&history))
            });
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        let source = r#"{"users":[{"name":"Ada"}],"big":123456789012345678901234567890}"#;
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input
                    .update(cx, |state, cx| state.replace_all(source, window, cx));
                view.set_mode(JsonMode::Query, window, cx);
                assert!(view.input.read(cx).value().contains('\n'));
                view.query
                    .update(cx, |state, cx| state.set_value("users[0].na", window, cx));
            })
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("tab");
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        workspace.read_with(&cx, |view, cx| {
            assert_eq!(view.query.read(cx).value().as_ref(), "users[0].name");
            assert_eq!(view.session.evaluation().output(), Some("Ada"));
            assert!(view.query_completions(cx).is_empty());
        });
        assert_eq!(history.load(Json::ID).unwrap().len(), 1);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                let snapshot = JsonSnapshot {
                    request: JsonRequest {
                        query: "big".into(),
                        ..JsonRequest::new(source, JsonMode::Query)
                    },
                    output: "123456789012345678901234567890".into(),
                };
                view.apply_restore(snapshot, window, cx);
                assert_eq!(view.input.read(cx).value().as_ref(), source);
                view.input
                    .update(cx, |state, cx| state.replace_all("{invalid}", window, cx));
            })
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        workspace.read_with(&cx, |view, cx| {
            assert_eq!(view.input.read(cx).value().as_ref(), "{invalid}");
            assert!(view.query_completions(cx).is_empty());
            assert!(!view.session.evaluation().diagnostics().is_empty());
        });
        assert_eq!(history.load(Json::ID).unwrap().len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn error_lines_use_source_bytes_including_unicode_and_blank_lines() {
        let source = "{\n  \"é\": 1,\n\n}";
        assert_eq!(&source[error_line_range(source, 2).unwrap()], "  \"é\": 1,");
        assert_eq!(error_line_range(source, 3), Some(13..13));
        assert_eq!(&source[error_line_range(source, 4).unwrap()], "}");
        assert_eq!(error_line_range(source, 0), None);
        assert_eq!(error_line_range(source, 5), None);
    }

    #[gpui::test]
    fn invalid_input_marks_the_parser_line_and_editing_clears_it(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| {
                JsonWorkspace::new(window, cx, Rc::new(TestClipboard), Rc::clone(&history))
            });
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        let source = "{\n  \"first\": 1\n  \"second\": 2\n}";
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input
                    .update(cx, |state, cx| state.replace_all(source, window, cx))
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        workspace.read_with(&cx, |view, cx| {
            assert_eq!(
                view.error_highlight.get_ranges(cx),
                vec![error_line_range(source, 3).unwrap()]
            );
            assert_eq!(view.input.read(cx).diagnostics().unwrap().len(), 1);
        });
        assert!(history.load(Json::ID).unwrap().is_empty());
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input
                    .update(cx, |state, cx| state.replace_all("{}", window, cx))
            });
            window.draw(cx).clear(cx);
        });
        workspace.read_with(&cx, |view, cx| {
            assert!(view.error_highlight.get_ranges(cx).is_empty());
            assert!(view.input.read(cx).diagnostics().unwrap().is_empty());
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        workspace.read_with(&cx, |view, cx| {
            assert_eq!(view.result.read(cx).value().as_ref(), "{}")
        });
        assert_eq!(history.load(Json::ID).unwrap().len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[gpui::test]
    fn selectable_history_and_confirmation_reconcile_after_settings_deletion(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| {
                JsonWorkspace::new(window, cx, Rc::new(TestClipboard), Rc::clone(&history))
            });
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.set_mode(JsonMode::Minify, window, cx));
            window.draw(cx).clear(cx);
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            JsonMode::Minify
        );
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.set_mode(JsonMode::Format, window, cx));
            window.draw(cx).clear(cx);
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            JsonMode::Format
        );
        for text in [r#"{"first":1}"#, r#"{"second":2}"#] {
            cx.update(|window, cx| {
                workspace.update(cx, |view, cx| {
                    view.input
                        .update(cx, |state, cx| state.replace_all(text, window, cx))
                });
                window.draw(cx).clear(cx);
            });
            cx.executor().advance_clock(DEBOUNCE);
            cx.run_until_parked();
        }
        let entries = history.load(Json::ID).unwrap();
        assert_eq!(entries.len(), 2);
        let first_id = entries[1].id.clone();
        let second_id = entries[0].id.clone();
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                assert!(view.history_view.select(&first_id));
                view.sync_history(cx);
                view.restore_selected(window, cx);
                assert!(view.history_view.pending_restore.is_some());
                view.cancel_restore(cx);
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(view.input.read(cx).value().to_string(), r#"{"second":2}"#);
            });
        });

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.restore_selected(window, cx);
                view.confirm_restore(window, cx);
                assert_eq!(view.input.read(cx).value().to_string(), r#"{"first":1}"#);
            });
        });
        assert_eq!(history.load(Json::ID).unwrap().len(), 2);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                assert!(view.history_view.select(&second_id));
                view.sync_history(cx);
                view.restore_selected(window, cx);
                assert!(view.history_view.pending_restore.is_some());
            });
        });
        cx.update(|_window, cx| history.clear_utility(Json::ID, cx).unwrap());
        assert!(workspace.read_with(&cx, |view, _| view.history_view.entries.is_empty()));
        assert!(workspace.read_with(&cx, |view, _| view.history_view.selected.is_none()));
        assert!(workspace.read_with(&cx, |view, _| view.history_view.pending_restore.is_none()));
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.input.read(cx).value().to_string()),
            r#"{"first":1}"#
        );
        fs::remove_dir_all(root).unwrap();
    }
}
