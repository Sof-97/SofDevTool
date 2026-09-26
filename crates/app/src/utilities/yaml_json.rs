//! The YAML/JSON workspace: convert one document in either direction, surface
//! fidelity limits and diagnostics, use an explicit Clipboard surface and fresh
//! Rust History.
//!
//! Input changes are debounced and revision-gated through the shared session, so
//! an obsolete asynchronous completion can never publish a result. One settled
//! valid operation is recorded once; preview and restore never reevaluate.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, Entity, IntoElement, Render, Subscription, Window};
use gpui_kit::component::{
    button::Button,
    input::{InputEvent, TextareaState},
    list::ListState,
    tab::{Tab, TabBar},
    ActiveTheme as _, Disableable as _,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::yaml_json::{
    YamlJson, YamlJsonDirection, YamlJsonEvaluation, YamlJsonRequest, YamlJsonSnapshot,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::ui;
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(250);

const LIMITATION_NOTICE: &str = "One document at a time. Comments, formatting and alias identity are not preserved; mapping order and exact key presentation may also change. Conversion is not lossless.";

type YamlJsonSession = Session<YamlJson>;

/// Builds the YAML/JSON workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| YamlJsonWorkspace::new(window, cx, clipboard, history))
        .into()
}

pub struct YamlJsonWorkspace {
    input: Entity<TextareaState>,
    result: Entity<TextareaState>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    direction: YamlJsonDirection,
    session: YamlJsonSession,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl YamlJsonWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let input = cx.new(|cx| TextareaState::new(window, cx));
        let result = cx.new(|cx| TextareaState::new(window, cx));
        let subscriptions = vec![cx.subscribe_in(
            &input,
            window,
            |this, _entity, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.schedule(window, cx);
                }
            },
        )];
        let history_view = HistoryViewState::load(&history, YamlJson::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(YamlJson::ID, move |cx| {
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
                        ui::history_set_selected(
                            &this.history_list,
                            this.history_view.selected.clone(),
                            cx,
                        );
                        cx.notify();
                    }
                })
                .ok();
            }),
        );
        let workspace = Self {
            input,
            result,
            clipboard,
            history,
            direction: YamlJsonDirection::YamlToJson,
            session: YamlJsonSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            suppress_changes: false,
            history_view,
            history_visible: true,
            history_list,
            _history_subscription: history_subscription,
            _subscriptions: subscriptions,
        };
        workspace.sync_history(cx);
        workspace
    }

    fn request(&self, cx: &App) -> YamlJsonRequest {
        YamlJsonRequest {
            input: self.input.read(cx).value().to_string(),
            direction: self.direction,
        }
    }

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

    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a YAML/JSON snapshot serializes");
        let result = self
            .history
            .record(YamlJson::ID, YamlJson::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, YamlJson::ID, result);
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, YamlJson::ID);
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
            YamlJsonEvaluation::Valid { output } => output.clone(),
            _ => String::new(),
        };
        self.result
            .update(cx, |state, cx| state.set_value(value, window, cx));
    }

    fn set_direction(
        &mut self,
        direction: YamlJsonDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.direction == direction {
            return;
        }
        self.direction = direction;
        self.copied = false;
        self.schedule(window, cx);
    }

    fn swap(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let previous_output = self
            .session
            .evaluation()
            .output()
            .map(str::to_owned)
            .unwrap_or_default();
        self.direction = self.direction.swapped();
        self.copied = false;
        if previous_output.is_empty() {
            self.schedule(window, cx);
        } else {
            self.input.update(cx, |state, cx| {
                state.replace_all(previous_output, window, cx)
            });
        }
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.input
                .update(cx, |state, cx| state.replace_all(text, window, cx));
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
        self.input
            .update(cx, |state, cx| state.replace_all("", window, cx));
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
        let current = self.input.read(cx).value().to_string();
        if !current.is_empty() && current != snapshot.request.input {
            self.history_view.pending_restore = Some(entry);
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current non-empty YAML/JSON session.",
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
        snapshot: YamlJsonSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.direction = snapshot.request.direction;
        self.input.update(cx, |state, cx| {
            state.set_value(snapshot.request.input.clone(), window, cx)
        });
        self.session.restore(snapshot);
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
            if self
                .history_view
                .retained(&self.history, YamlJson::ID, &entry)
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
                .retained(&self.history, YamlJson::ID, &entry)
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
                    selectable: true,
                }
            })
            .collect()
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

        let restore = Button::new("yaml-json.history.restore-selected")
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

impl Render for YamlJsonWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let theme = cx.theme().clone();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), YamlJsonEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.is_empty())
                .unwrap_or(false);

        let input_title = match self.direction {
            YamlJsonDirection::YamlToJson => "YAML Input",
            YamlJsonDirection::JsonToYaml => "JSON Input",
        };
        let result_title = match self.direction {
            YamlJsonDirection::YamlToJson => "JSON Result",
            YamlJsonDirection::JsonToYaml => "YAML Result",
        };

        let direction_index = match self.direction {
            YamlJsonDirection::YamlToJson => 0,
            YamlJsonDirection::JsonToYaml => 1,
        };
        let direction_control = TabBar::new("yaml-json.direction")
            .segmented()
            .selected_index(direction_index)
            .children([
                Tab::new().label("YAML → JSON"),
                Tab::new().label("JSON → YAML"),
            ])
            .on_click(cx.listener(|this, index, window, cx| {
                let direction = match index {
                    0 => YamlJsonDirection::YamlToJson,
                    _ => YamlJsonDirection::JsonToYaml,
                };
                this.set_direction(direction, window, cx);
            }));

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(direction_control)
            .child(
                Button::new("yaml-json.swap")
                    .label("Swap")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.swap(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(ui::copy_feedback(cx, self.copied, "Copied to Clipboard"))
            .child(
                Button::new("yaml-json.history.toggle")
                    .label(if self.history_visible {
                        "History: on"
                    } else {
                        "History: off"
                    })
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.history_visible = !this.history_visible;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("yaml-json.paste")
                    .label("Paste")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::new("yaml-json.copy-result")
                    .label("Copy Result")
                    .disabled(!can_copy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::new("yaml-json.clear")
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
            .bg(theme.background)
            .text_color(theme.foreground)
            .gap_3()
            .child(toolbar)
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(LIMITATION_NOTICE),
            );

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("YAML/JSON History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            ui::empty_state(cx, "Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), YamlJsonEvaluation::Empty) {
            ui::empty_state(cx, "Paste or type a document to begin").into_any_element()
        } else {
            ui::multiline_editor(&self.result, true, "yaml-json.result").into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(ui::panel(
                cx,
                input_title,
                "one document",
                ui::multiline_editor(&self.input, false, "yaml-json.input"),
            ))
            .child(ui::panel(
                cx,
                result_title,
                "read-only, selectable",
                result_body,
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics(cx))
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<YamlJsonSnapshot> {
    if entry.snapshot_version != YamlJson::SNAPSHOT_VERSION {
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

#[cfg(test)]
mod history_tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use gpui_kit::component::Root;

    use crate::history::{HistoryStore, SystemClock};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestClipboard;

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            None
        }

        fn write_text(&self, _text: &str, _cx: &mut App) {}
    }

    fn test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-yaml-history-{}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn recorder(root: &Path) -> Rc<HistoryRecorder> {
        Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.to_path_buf()),
            Box::new(SystemClock::new()),
        ))
    }

    fn snapshot(input: &str, output: &str) -> YamlJsonSnapshot {
        YamlJsonSnapshot {
            request: YamlJsonRequest::new(input, YamlJsonDirection::YamlToJson),
            output: output.into(),
        }
    }

    fn entry(id: &str, snapshot: YamlJsonSnapshot) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            captured_at: "2026-01-01T00:00:00Z".into(),
            utility_id: YamlJson::ID.into(),
            snapshot_version: YamlJson::SNAPSHOT_VERSION,
            payload: serde_json::to_value(snapshot).unwrap(),
        }
    }

    #[gpui::test]
    fn clear_reaches_visible_and_hidden_workspaces_without_changing_sessions(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let root = test_root();
        let history = recorder(&root);
        let retained = entry("retained", snapshot("old: value", "{\"old\":\"value\"}"));
        history.store().record(retained.clone()).unwrap();
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard);
        let mut captured = None;
        let (_root_view, cx) = cx.add_window_view(|window, cx| {
            let view =
                cx.new(|cx| YamlJsonWorkspace::new(window, cx, clipboard.clone(), history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let visible = captured.unwrap();
        let hidden = cx.update(|window, cx| {
            cx.new(|cx| YamlJsonWorkspace::new(window, cx, clipboard, history.clone()))
        });
        let current = snapshot("current: value", "{\"current\":\"value\"}");
        cx.update(|window, cx| {
            for workspace in [&visible, &hidden] {
                workspace.update(cx, |view, cx| {
                    view.input.update(cx, |state, cx| {
                        state.set_value(current.request.input.clone(), window, cx)
                    });
                    view.session.restore(current.clone());
                    view.sync_display(window, cx);
                    assert!(view.history_view.select(&retained.id));
                    view.history_view.pending_restore = Some(retained.clone());
                });
            }
            visible.update(cx, |view, cx| {
                view.request_restore(retained.clone(), window, cx);
                assert_eq!(view.history_view.pending_restore, Some(retained.clone()));
                view.confirm_restore(window, cx);
                assert_eq!(view.input.read(cx).value().to_string(), "old: value");
                assert_eq!(
                    view.result.read(cx).value().to_string(),
                    "{\"old\":\"value\"}"
                );
                assert!(view.history_view.pending_restore.is_none());
                view.input.update(cx, |state, cx| {
                    state.set_value(current.request.input.clone(), window, cx)
                });
                view.session.restore(current.clone());
                view.sync_display(window, cx);
                view.history_view.pending_restore = Some(retained.clone());
            });
            assert_eq!(history.load(YamlJson::ID).unwrap().len(), 1);
            history.clear_utility(YamlJson::ID, cx).unwrap();
            for workspace in [&visible, &hidden] {
                let view = workspace.read(cx);
                assert!(view.history_view.entries.is_empty());
                assert!(view.history_view.selected.is_none());
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(
                    view.input.read(cx).value().to_string(),
                    current.request.input
                );
                assert_eq!(
                    view.session.evaluation().output(),
                    Some(current.output.as_str())
                );
                assert_eq!(view.result.read(cx).value().to_string(), current.output);
            }
            // A late confirmation or row click still checks persisted retention.
            visible.update(cx, |view, cx| {
                view.history_view.pending_restore = Some(retained.clone());
                view.confirm_restore(window, cx);
                view.history_view.entries.push(retained.clone());
                view.history_view.selected = Some(retained.id.clone());
                view.restore_selected(window, cx);
                assert_eq!(
                    view.input.read(cx).value().to_string(),
                    current.request.input
                );
                assert!(view.history_view.selected.is_none());
                assert!(view.history_view.pending_restore.is_none());
            });

            // A failed deletion for an unknown file does not keep a cleared
            // Utility's stale rows alive in either open workspace.
            let later = entry("later", snapshot("later: value", "{\"later\":\"value\"}"));
            history.store().record(later.clone()).unwrap();
            fs::create_dir(root.join("legacy-unknown.history.v1.json")).unwrap();
            for workspace in [&visible, &hidden] {
                workspace.update(cx, |view, cx| {
                    view.reconcile_history(cx);
                    assert!(view.history_view.select(&later.id));
                    view.history_view.pending_restore = Some(later.clone());
                });
            }
            let report = history.clear_all(cx).unwrap();
            assert_eq!(report.cleared_ids, vec![YamlJson::ID]);
            assert_eq!(report.failed_ids, vec!["legacy-unknown"]);
            for workspace in [&visible, &hidden] {
                let view = workspace.read(cx);
                assert!(view.history_view.entries.is_empty());
                assert!(view.history_view.selected.is_none());
                assert!(view.history_view.pending_restore.is_none());
                assert_eq!(
                    view.input.read(cx).value().to_string(),
                    current.request.input
                );
                assert_eq!(view.result.read(cx).value().to_string(), current.output);
            }
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn failed_recording_and_corruption_warn_without_overwriting_or_changing_output(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let root = test_root();
        let history = recorder(&root);
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard);
        let (workspace, cx) = cx.add_window_view(|window, cx| {
            YamlJsonWorkspace::new(window, cx, clipboard, history.clone())
        });
        history.store().fail_next_writes(1);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                let request = YamlJsonRequest::new("good: value", YamlJsonDirection::YamlToJson);
                view.input.update(cx, |state, cx| {
                    state.set_value(request.input.clone(), window, cx)
                });
                let SubmitOutcome::Scheduled(revision) = view.session.submit(request) else {
                    panic!("new request must schedule");
                };
                view.session.resolve(revision).unwrap();
                view.sync_display(window, cx);
                let output = view.result.read(cx).value().to_string();
                view.record_settled(cx);
                assert_eq!(view.result.read(cx).value().to_string(), output);
                assert!(view.history_view.error.is_some());
                assert!(view.history_view.entries.is_empty());
                let next = YamlJsonRequest::new("next: value", YamlJsonDirection::YamlToJson);
                view.input.update(cx, |state, cx| {
                    state.set_value(next.input.clone(), window, cx)
                });
                let SubmitOutcome::Scheduled(revision) = view.session.submit(next) else {
                    panic!("new request must schedule");
                };
                view.session.resolve(revision).unwrap();
                view.sync_display(window, cx);
                view.record_settled(cx);
                assert!(view.history_view.error.is_some());
                assert!(!view.result.read(cx).value().is_empty());
            });
            assert!(history.store().is_paused(YamlJson::ID));
            assert!(history.load(YamlJson::ID).unwrap().is_empty());
            history.retry_recording(YamlJson::ID, cx).unwrap();
            let view = workspace.read(cx);
            assert!(view.history_view.error.is_none());
            assert!(view.history_view.entries.is_empty());
            assert_eq!(view.input.read(cx).value().to_string(), "next: value");
            assert!(!view.result.read(cx).value().is_empty());
        });
        let path = root.join(format!("{}.history.v1.json", YamlJson::ID));
        fs::write(&path, b"not JSON").unwrap();
        cx.update(|_, cx| {
            history.retry_recording(YamlJson::ID, cx).unwrap_err();
            let view = workspace.read(cx);
            assert!(view.history_view.error.is_some());
            assert!(view.history_view.entries.is_empty());
            assert_eq!(view.input.read(cx).value().to_string(), "next: value");
            assert!(!view.result.read(cx).value().is_empty());
        });
        assert_eq!(fs::read(path).unwrap(), b"not JSON");
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use gpui::VisualTestContext;
    use gpui_kit::component::Root;

    use crate::history::{HistoryStore, SystemClock};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct TestClipboard(RefCell<Option<String>>);

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            self.0.borrow().clone()
        }

        fn write_text(&self, text: &str, _cx: &mut App) {
            *self.0.borrow_mut() = Some(text.to_owned());
        }
    }

    #[gpui::test]
    fn keyboard_direction_and_copy_track_current_valid_result(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_kit::init);
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-yaml-redesign-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let clipboard = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| YamlJsonWorkspace::new(window, cx, clipboard.clone(), history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input.update(cx, |state, cx| {
                    state.replace_all("message: café", window, cx)
                })
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        let json = workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string());
        assert!(json.contains("café"));
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_result(cx));
        });
        assert_eq!(clipboard.0.borrow().as_deref(), Some(json.as_str()));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.set_direction(YamlJsonDirection::JsonToYaml, window, cx)
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.direction),
            YamlJsonDirection::JsonToYaml
        );
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert!(!workspace.read_with(&cx, |view, _| view
            .session
            .evaluation()
            .is_valid_operation()));
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_result(cx));
        });
        assert_eq!(clipboard.0.borrow().as_deref(), Some(json.as_str()));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input.update(cx, |state, cx| {
                    state.replace_all(r#"{"message":"café"}"#, window, cx)
                })
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert!(workspace
            .read_with(&cx, |view, cx| view.result.read(cx).value().to_string())
            .contains("café"));
        assert_eq!(history.load(YamlJson::ID).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
