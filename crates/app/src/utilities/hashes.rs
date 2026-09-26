//! The Hashes workspace: choose an algorithm and representation, then
//! explicitly Hash the exact UTF-8 bytes with a fresh Rust History.
//!
//! Hashing is a deliberate action rather than live evaluation. Editing the
//! input, algorithm or representation invalidates the visible digest so no
//! stale result remains; pressing Hash recomputes and records exactly one
//! settled valid operation, including for empty input.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, AnyView, App, Context, Entity, IntoElement, Render, Subscription, Window};
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    input::{InputEvent, TextareaState},
    list::ListState,
    tab::{Tab, TabBar},
    ActiveTheme as _, Disableable as _,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::hashes::{
    HashAlgorithm, HashRepresentation, Hashes, HashesEvaluation, HashesRequest, HashesSnapshot,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::ui;
use crate::workbench::Workbench;

type HashesSession = Session<Hashes>;

/// Builds the Hashes workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| HashesWorkspace::new(window, cx, clipboard, history))
        .into()
}

pub struct HashesWorkspace {
    input: Entity<TextareaState>,
    result: Entity<TextareaState>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    algorithm: HashAlgorithm,
    representation: HashRepresentation,
    session: HashesSession,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl HashesWorkspace {
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
                    this.invalidate(window, cx);
                }
            },
        )];
        let history_view = HistoryViewState::load(&history, Hashes::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Hashes::ID, move |cx| {
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
            algorithm: HashAlgorithm::Sha256,
            representation: HashRepresentation::LowercaseHex,
            session: HashesSession::new(),
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

    fn request(&self, cx: &gpui::App) -> HashesRequest {
        HashesRequest {
            input: self.input.read(cx).value().to_string(),
            algorithm: self.algorithm,
            representation: self.representation,
        }
    }

    /// Discards the visible digest after any input, algorithm or representation
    /// change, so a result never appears to describe the current input.
    fn invalidate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.suppress_changes {
            return;
        }
        self.copied = false;
        self.session.clear();
        self.display_epoch = u64::MAX;
        self.sync_display(window, cx);
        cx.notify();
    }

    /// An explicit Hash: a fresh revision is forced so a deliberate repeat
    /// records separately, then resolved and recorded synchronously.
    fn hash(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = false;
        self.session.clear();
        let SubmitOutcome::Scheduled(revision) = self.session.submit(self.request(cx)) else {
            return;
        };
        self.session.resolve(revision);
        self.display_epoch = u64::MAX;
        self.sync_display(window, cx);
        self.record_settled(cx);
        cx.notify();
    }

    fn record_settled(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.session.take_snapshot() else {
            return;
        };
        let payload = serde_json::to_value(&snapshot).expect("a Hashes snapshot serializes");
        let result = self
            .history
            .record(Hashes::ID, Hashes::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Hashes::ID, result);
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Hashes::ID);
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
            HashesEvaluation::Valid { output } => output.clone(),
            _ => String::new(),
        };
        self.result
            .update(cx, |state, cx| state.set_value(value, window, cx));
    }

    fn set_algorithm(
        &mut self,
        algorithm: HashAlgorithm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.algorithm == algorithm {
            return;
        }
        self.algorithm = algorithm;
        self.invalidate(window, cx);
    }

    fn set_representation(
        &mut self,
        representation: HashRepresentation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.representation == representation {
            return;
        }
        self.representation = representation;
        self.invalidate(window, cx);
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
                "Restoring this History entry replaces the current non-empty Hashes session.",
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
        snapshot: HashesSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.algorithm = snapshot.request.algorithm;
        self.representation = snapshot.request.representation;
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
                .retained(&self.history, Hashes::ID, &entry)
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
                .retained(&self.history, Hashes::ID, &entry)
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
                        .map(|snapshot| {
                            format!(
                                "{} · {}",
                                snapshot.request.algorithm.label(),
                                preview_line(&snapshot.output)
                            )
                        })
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
        let restore = Button::new("hashes.history.restore-selected")
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

impl Render for HashesWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let theme = cx.theme().clone();
        let can_copy = self.session.evaluation().is_valid_operation();

        let algorithm_index = HashAlgorithm::ALL
            .iter()
            .position(|choice| *choice == self.algorithm)
            .unwrap_or(0);
        let algorithm_control = TabBar::new("hashes.algorithm")
            .segmented()
            .selected_index(algorithm_index)
            .children(
                HashAlgorithm::ALL
                    .into_iter()
                    .map(|choice| Tab::new().label(choice.label())),
            )
            .on_click(cx.listener(|this, index, window, cx| {
                if let Some(choice) = HashAlgorithm::ALL.get(*index) {
                    this.set_algorithm(*choice, window, cx);
                }
            }));
        let representation_index = HashRepresentation::ALL
            .iter()
            .position(|choice| *choice == self.representation)
            .unwrap_or(0);
        let representation_control = TabBar::new("hashes.representation")
            .segmented()
            .selected_index(representation_index)
            .children(
                HashRepresentation::ALL
                    .into_iter()
                    .map(|choice| Tab::new().label(choice.label())),
            )
            .on_click(cx.listener(|this, index, window, cx| {
                if let Some(choice) = HashRepresentation::ALL.get(*index) {
                    this.set_representation(*choice, window, cx);
                }
            }));

        let algorithm_toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(algorithm_control)
            .child(representation_control)
            .child(
                Button::new("hashes.hash")
                    .label("Hash")
                    .primary()
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.hash(window, cx);
                    })),
            );

        let actions_toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(div().flex_1())
            .child(ui::copy_feedback(cx, self.copied, "Copied to Clipboard"))
            .child(
                Button::new("hashes.history.toggle")
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
                Button::new("hashes.paste")
                    .label("Paste")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::new("hashes.copy-result")
                    .label("Copy Result")
                    .disabled(!can_copy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::new("hashes.clear")
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
            .child(algorithm_toolbar)
            .child(actions_toolbar)
            .child(div().text_xs().text_color(theme.muted_foreground).child(
                "Hashes the exact UTF-8 bytes. Files, HMAC, and password hashing are not \
                     included.",
            ));

        if self.algorithm.is_legacy() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                self.algorithm.notice(),
                None,
            ));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Hashes History: {error}"),
                None,
            ));
        }

        let result_body = if matches!(self.session.evaluation(), HashesEvaluation::Valid { .. }) {
            ui::multiline_editor(&self.result, true, "hashes.result").into_any_element()
        } else {
            ui::empty_state(cx, "Press Hash to compute the digest").into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(ui::panel(
                cx,
                "UTF-8 Input",
                "exact bytes",
                ui::multiline_editor(&self.input, false, "hashes.input"),
            ))
            .child(ui::panel(
                cx,
                "Digest",
                "read-only, selectable",
                result_body,
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics(cx))
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<HashesSnapshot> {
    if entry.snapshot_version != Hashes::SNAPSHOT_VERSION {
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
mod interaction_tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use gpui::{App, VisualTestContext};
    use gpui_kit::component::Root;

    use crate::history::{HistoryStore, SystemClock};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const EMPTY_SHA1: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";

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
    fn explicit_empty_hash_legacy_choice_copy_and_exact_restore(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_kit::init);
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-hashes-redesign-{}-{}",
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
                cx.new(|cx| HashesWorkspace::new(window, cx, clipboard.clone(), history.clone()));
            captured = Some(view.clone());
            Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.hash(window, cx));
        });
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            EMPTY_SHA256
        );
        let entries = history.load(Hashes::ID).unwrap();
        assert_eq!(entries.len(), 1, "explicit empty bytes are an operation");

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.set_algorithm(HashAlgorithm::Sha1, window, cx)
            });
        });
        assert!(
            workspace
                .read_with(&cx, |view, cx| view.result.read(cx).value().to_string())
                .is_empty(),
            "changing the algorithm invalidates the visible digest"
        );
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.hash(window, cx));
        });
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            EMPTY_SHA1
        );
        assert_eq!(history.load(Hashes::ID).unwrap().len(), 2);
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_result(cx));
        });
        assert_eq!(clipboard.0.borrow().as_deref(), Some(EMPTY_SHA1));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input
                    .update(cx, |state, cx| state.replace_all("other", window, cx))
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                assert!(view.history_view.select(&entries[0].id));
                view.restore_selected(window, cx);
                assert!(view.history_view.pending_restore.is_some());
                view.confirm_restore(window, cx);
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.algorithm),
            HashAlgorithm::Sha256
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            EMPTY_SHA256
        );
        assert_eq!(
            history.load(Hashes::ID).unwrap().len(),
            2,
            "restore must not record"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
