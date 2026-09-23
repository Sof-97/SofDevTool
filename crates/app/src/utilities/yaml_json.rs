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
use gpui::{div, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::yaml_json::{
    YamlJson, YamlJsonDirection, YamlJsonEvaluation, YamlJsonRequest, YamlJsonSnapshot,
};
use sofdevtool_core::utility::Utility;
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    DiagnosticSeverity, HistoryItem, HistoryPanel, TextEditor, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
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

struct ButtonFocus {
    yaml_to_json: FocusHandle,
    json_to_yaml: FocusHandle,
    swap: FocusHandle,
    paste: FocusHandle,
    copy: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
}

pub struct YamlJsonWorkspace {
    input: TextEditor,
    result: TextEditor,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    direction: YamlJsonDirection,
    session: YamlJsonSession,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    _history_subscription: HistorySubscription,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl YamlJsonWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let input = TextEditor::new(window, cx);
        let result = TextEditor::new(window, cx);
        let subscriptions = vec![input.on_change_in(window, cx, |this, window, cx| {
            this.schedule(window, cx);
        })];
        let history_view = HistoryViewState::load(&history, YamlJson::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(YamlJson::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        Self {
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
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                yaml_to_json: cx.focus_handle().tab_stop(true).tab_index(0),
                json_to_yaml: cx.focus_handle().tab_stop(true).tab_index(0),
                swap: cx.focus_handle().tab_stop(true).tab_index(0),
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

    fn request(&self, cx: &App) -> YamlJsonRequest {
        YamlJsonRequest {
            input: self.input.text(cx),
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
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, YamlJson::ID);
        cx.notify();
    }

    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            YamlJsonEvaluation::Valid { output } => {
                self.result.set_text(output.clone(), window, cx);
            }
            _ => self.result.set_text("", window, cx),
        }
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
            self.input.replace_all(previous_output, window, cx);
        }
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            self.input.replace_all(text, window, cx);
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
        self.input.replace_all("", window, cx);
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
        let current = self.input.text(cx);
        if !current.is_empty() && current != snapshot.request.input {
            self.history_view.pending_restore = Some(entry);
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
        self.input
            .set_text(snapshot.request.input.clone(), window, cx);
        self.session.restore(snapshot);
        self.display_epoch = u64::MAX;
        self.suppress_changes = false;
        self.history_view.pending_restore = None;
        self.copied = false;
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
                        .map(|snapshot| preview_line(&snapshot.output))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn direction_button(
        &self,
        direction: YamlJsonDirection,
        focus: FocusHandle,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(direction.label())
            .id(match direction {
                YamlJsonDirection::YamlToJson => "yaml-json.direction.yaml-to-json",
                YamlJsonDirection::JsonToYaml => "yaml-json.direction.json-to-yaml",
            })
            .variant(if self.direction == direction {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(focus)
            .on_click(view_click(cx, move |this, window, cx| {
                this.set_direction(direction, window, cx);
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
            .child(div().text_xs().text_color(tokens.text()).child(
                "Restoring this History entry replaces the current non-empty YAML/JSON session.",
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

impl Render for YamlJsonWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
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

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(self.direction_button(
                YamlJsonDirection::YamlToJson,
                self.focus.yaml_to_json.clone(),
                cx,
            ))
            .child(self.direction_button(
                YamlJsonDirection::JsonToYaml,
                self.focus.json_to_yaml.clone(),
                cx,
            ))
            .child(
                Button::new("Swap")
                    .focus_handle(self.focus.swap.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.swap(window, cx);
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
                            .child("YAML / JSON"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("YAML 1.2 Core, local and offline"),
                    ),
            )
            .child(toolbar)
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child(LIMITATION_NOTICE),
            );

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("YAML/JSON History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            empty_state("Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), YamlJsonEvaluation::Empty) {
            empty_state("Paste or type a document to begin").into_any_element()
        } else {
            self.result
                .render(true, "yaml-json.result")
                .into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_4()
            .flex_1()
            .min_h_0()
            .child(panel(
                input_title,
                "one document",
                self.input.render(false, "yaml-json.input"),
            ))
            .child(panel(result_title, "read-only, selectable", result_body));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics())
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

    use crate::history::{HistoryStore, SystemClock};

    struct TestClipboard;

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            None
        }

        fn write_text(&self, _text: &str, _cx: &mut App) {}
    }

    fn test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-yaml-history-{}-{}",
            std::process::id(),
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
        cx.update(sofdevtool_ui::init);
        let root = test_root();
        let history = recorder(&root);
        let retained = entry("retained", snapshot("old: value", "{\"old\":\"value\"}"));
        history.store().record(retained.clone()).unwrap();
        let clipboard: Rc<dyn Clipboard> = Rc::new(TestClipboard);
        let (visible, cx) = cx.add_window_view(|window, cx| {
            YamlJsonWorkspace::new(window, cx, clipboard.clone(), history.clone())
        });
        let hidden = cx.update(|window, cx| {
            cx.new(|cx| YamlJsonWorkspace::new(window, cx, clipboard, history.clone()))
        });
        let current = snapshot("current: value", "{\"current\":\"value\"}");
        cx.update(|window, cx| {
            for workspace in [&visible, &hidden] {
                workspace.update(cx, |view, cx| {
                    view.input
                        .assign_text(current.request.input.clone(), window, cx);
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
                assert_eq!(view.input.text(cx), "old: value");
                assert_eq!(view.result.text(cx), "{\"old\":\"value\"}");
                assert!(view.history_view.pending_restore.is_none());
                view.input
                    .assign_text(current.request.input.clone(), window, cx);
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
                assert_eq!(view.input.text(cx), current.request.input);
                assert_eq!(
                    view.session.evaluation().output(),
                    Some(current.output.as_str())
                );
                assert_eq!(view.result.text(cx), current.output);
            }
            // A late confirmation or row click still checks persisted retention.
            visible.update(cx, |view, cx| {
                view.history_view.pending_restore = Some(retained.clone());
                view.confirm_restore(window, cx);
                view.history_view.entries.push(retained.clone());
                view.history_view.selected = Some(retained.id.clone());
                view.restore_selected(window, cx);
                assert_eq!(view.input.text(cx), current.request.input);
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
                assert_eq!(view.input.text(cx), current.request.input);
                assert_eq!(view.result.text(cx), current.output);
            }
        });
        fs::remove_dir_all(root).unwrap();
    }

    #[gpui::test]
    fn failed_recording_and_corruption_warn_without_overwriting_or_changing_output(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(sofdevtool_ui::init);
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
                view.input.assign_text(request.input.clone(), window, cx);
                let SubmitOutcome::Scheduled(revision) = view.session.submit(request) else {
                    panic!("new request must schedule");
                };
                view.session.resolve(revision).unwrap();
                view.sync_display(window, cx);
                let output = view.result.text(cx);
                view.record_settled(cx);
                assert_eq!(view.result.text(cx), output);
                assert!(view.history_view.error.is_some());
                assert!(view.history_view.entries.is_empty());
                let next = YamlJsonRequest::new("next: value", YamlJsonDirection::YamlToJson);
                view.input.assign_text(next.input.clone(), window, cx);
                let SubmitOutcome::Scheduled(revision) = view.session.submit(next) else {
                    panic!("new request must schedule");
                };
                view.session.resolve(revision).unwrap();
                view.sync_display(window, cx);
                view.record_settled(cx);
                assert!(view.history_view.error.is_some());
                assert!(!view.result.text(cx).is_empty());
            });
            assert!(history.store().is_paused(YamlJson::ID));
            assert!(history.load(YamlJson::ID).unwrap().is_empty());
            history.retry_recording(YamlJson::ID, cx).unwrap();
            let view = workspace.read(cx);
            assert!(view.history_view.error.is_none());
            assert!(view.history_view.entries.is_empty());
            assert_eq!(view.input.text(cx), "next: value");
            assert!(!view.result.text(cx).is_empty());
        });
        let path = root.join(format!("{}.history.v1.json", YamlJson::ID));
        fs::write(&path, b"not JSON").unwrap();
        cx.update(|_, cx| {
            history.retry_recording(YamlJson::ID, cx).unwrap_err();
            let view = workspace.read(cx);
            assert!(view.history_view.error.is_some());
            assert!(view.history_view.entries.is_empty());
            assert_eq!(view.input.text(cx), "next: value");
            assert!(!view.result.text(cx).is_empty());
        });
        assert_eq!(fs::read(path).unwrap(), b"not JSON");
        fs::remove_dir_all(root).unwrap();
    }
}
