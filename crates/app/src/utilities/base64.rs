//! The Base64 workspace: encode/decode UTF-8 with explicit alphabet and padding
//! controls, an explicit Clipboard surface and fresh Rust History.

use std::rc::Rc;
use std::time::Duration;

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
use sofdevtool_core::utilities::base64::{
    Base64, Base64Alphabet, Base64Evaluation, Base64Mode, Base64Request, Base64Snapshot,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::ui;
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(200);

type Base64Session = Session<Base64>;

/// Builds the Base64 workspace as a type-erased view for the Workbench.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| Base64Workspace::new(window, cx, clipboard, history))
        .into()
}

pub struct Base64Workspace {
    input: Entity<TextareaState>,
    result: Entity<TextareaState>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    mode: Base64Mode,
    alphabet: Base64Alphabet,
    padded: bool,
    session: Base64Session,
    display_epoch: u64,
    copied: bool,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl Base64Workspace {
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
        let history_view = HistoryViewState::load(&history, Base64::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(Base64::ID, move |cx| {
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
            clipboard,
            history,
            mode: Base64Mode::Encode,
            alphabet: Base64Alphabet::Standard,
            padded: true,
            session: Base64Session::new(),
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

    fn request(&self, cx: &App) -> Base64Request {
        Base64Request {
            input: self.input.read(cx).value().to_string(),
            mode: self.mode,
            alphabet: self.alphabet,
            padded: self.padded,
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
        let payload = serde_json::to_value(&snapshot).expect("a Base64 snapshot serializes");
        let result = self
            .history
            .record(Base64::ID, Base64::SNAPSHOT_VERSION, payload);
        self.history_view
            .apply_record(&self.history, Base64::ID, result);
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view.reconcile(&self.history, Base64::ID);
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
            Base64Evaluation::Valid { output } => output.clone(),
            _ => String::new(),
        };
        self.result
            .update(cx, |state, cx| state.set_value(value, window, cx));
    }

    fn set_mode(&mut self, mode: Base64Mode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        self.schedule(window, cx);
    }

    fn set_alphabet(
        &mut self,
        alphabet: Base64Alphabet,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.alphabet = alphabet;
        self.schedule(window, cx);
    }

    fn toggle_padding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.padded = !self.padded;
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
                "Restoring this History entry replaces the current non-empty Base64 session.",
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
        snapshot: Base64Snapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.mode = snapshot.request.mode;
        self.alphabet = snapshot.request.alphabet;
        self.padded = snapshot.request.padded;
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
            let retained = self
                .history_view
                .retained(&self.history, Base64::ID, &entry);
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
            if self
                .history_view
                .retained(&self.history, Base64::ID, &entry)
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
                    status: (!snapshot.is_some()).then(|| "Unavailable".to_owned()),
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
        let restore = Button::new("base64.history.restore-selected")
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

impl Render for Base64Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let theme = cx.theme().clone();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), Base64Evaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.is_empty())
                .unwrap_or(false);

        let selected_mode = if self.mode == Base64Mode::Encode {
            0
        } else {
            1
        };
        let mode_control = TabBar::new("base64.mode")
            .segmented()
            .selected_index(selected_mode)
            .children([Tab::new().label("Encode"), Tab::new().label("Decode")])
            .on_click(cx.listener(|this, index, window, cx| {
                let mode = if *index == 0 {
                    Base64Mode::Encode
                } else {
                    Base64Mode::Decode
                };
                this.set_mode(mode, window, cx);
            }));
        let selected_alphabet = if self.alphabet == Base64Alphabet::Standard {
            0
        } else {
            1
        };
        let alphabet_control = TabBar::new("base64.alphabet")
            .segmented()
            .selected_index(selected_alphabet)
            .children([Tab::new().label("Standard"), Tab::new().label("URL-safe")])
            .on_click(cx.listener(|this, index, window, cx| {
                let alphabet = if *index == 0 {
                    Base64Alphabet::Standard
                } else {
                    Base64Alphabet::UrlSafe
                };
                this.set_alphabet(alphabet, window, cx);
            }));
        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(mode_control)
            .child(alphabet_control)
            .child(
                Button::new("base64.padding")
                    .label(if self.padded {
                        "Padding: on"
                    } else {
                        "Padding: off"
                    })
                    .when(self.padded, |button| button.primary())
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.toggle_padding(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(ui::copy_feedback(cx, self.copied, "Copied to Clipboard"))
            .child(
                Button::new("base64.history.toggle")
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
                Button::new("base64.paste")
                    .label("Paste")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::new("base64.copy-result")
                    .label("Copy Result")
                    .disabled(!can_copy)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.copy_result(cx);
                    })),
            )
            .child(
                Button::new("base64.clear")
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
            .child(toolbar);

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Base64 History: {error}"),
                None,
            ));
        }

        let result_body = if pending {
            ui::empty_state(cx, "Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), Base64Evaluation::Empty) {
            ui::empty_state(cx, "Paste or type text to begin").into_any_element()
        } else {
            ui::multiline_editor(&self.result, true, "base64.result").into_any_element()
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
                ui::multiline_editor(&self.input, false, "base64.input"),
            ))
            .child(ui::panel(
                cx,
                "Result",
                "read-only, selectable",
                result_body,
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics(cx))
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<Base64Snapshot> {
    if entry.snapshot_version != Base64::SNAPSHOT_VERSION {
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
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use gpui::{Focusable as _, VisualTestContext};
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

    fn isolated_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-base64-redesign-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    // Match the Workbench's sized flex host so List hit testing uses visible bounds.
    struct TestWorkspaceHost(Entity<Base64Workspace>);

    impl Render for TestWorkspaceHost {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().flex().child(self.0.clone())
        }
    }

    #[gpui::test]
    fn keyboard_mode_copy_invalid_and_exact_history_restore(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_kit::init);
        let root = isolated_root();
        let history = Rc::new(HistoryRecorder::new(
            HistoryStore::new(root.clone()),
            Box::new(SystemClock::new()),
        ));
        let clipboard = Rc::new(TestClipboard::default());
        let mut captured = None;
        let window = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| Base64Workspace::new(window, cx, clipboard.clone(), history.clone()));
            captured = Some(view.clone());
            let host = cx.new(|_| TestWorkspaceHost(view));
            Root::new(host, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.simulate_resize(gpui::size(gpui::px(2400.), gpui::px(900.)));

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input
                    .update(cx, |state, cx| state.replace_all("café", window, cx))
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            "Y2Fmw6k="
        );
        let first_entry = history.load(Base64::ID).unwrap().pop().unwrap();

        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_result(cx));
        });
        assert_eq!(
            clipboard.0.borrow().as_deref(),
            Some("Y2Fmw6k="),
            "the explicit Copy Result action copies the digest"
        );

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| view.set_mode(Base64Mode::Decode, window, cx));
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            Base64Mode::Decode
        );

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.input
                    .update(cx, |state, cx| state.replace_all("%%%", window, cx))
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert!(!workspace.read_with(&cx, |view, _| view
            .session
            .evaluation()
            .is_valid_operation()));
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy_result(cx));
        });
        assert_eq!(
            clipboard.0.borrow().as_deref(),
            Some("Y2Fmw6k="),
            "copying an invalid operation does not overwrite the clipboard"
        );

        // The multiline input keeps grapheme-safe editing and reaches the
        // workspace through its own focus handle; type and delete a combining
        // sequence to prove the kit control still drives `schedule`.
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.set_mode(Base64Mode::Encode, window, cx);
                view.input
                    .update(cx, |state, cx| state.replace_all("", window, cx));
            });
            window.draw(cx).clear(cx);
            let input = workspace.read(cx).input.clone();
            let focus = input.read(cx).focus_handle(cx).clone();
            window.focus(&focus, cx);
        });
        cx.simulate_input("cafe\u{301}");
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.input.read(cx).value().to_string()),
            "cafe\u{301}"
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            "Y2FmZcyB",
            "the grapheme-safe input observed the combining sequence"
        );

        // A single Backspace removes the whole combining grapheme, not just the
        // trailing accent: the app-owned editor intercepts the kit action.
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let input = workspace.read(cx).input.clone();
            let focus = input.read(cx).focus_handle(cx).clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("backspace");
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.input.read(cx).value().to_string()),
            "caf",
            "one Backspace removes the whole base-and-combining grapheme"
        );

        // Drive the real kit List selection path, including its delegate callback.
        // Direct HistoryViewState::select calls miss a recursive entity update.
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let list = workspace.read(cx).history_list.clone();
            window.focus(&list.read(cx).focus_handle(cx), cx);
        });
        cx.simulate_keystrokes("down");
        let latest_id = workspace.read_with(&cx, |view, _| view.history_view.entries[0].id.clone());
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.history_view.selected.clone()),
            Some(latest_id.clone())
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let row = cx
            .debug_bounds(Box::leak(
                format!("history.entry.{}", first_entry.id).into_boxed_str(),
            ))
            .expect("retained row is rendered");
        cx.simulate_click(row.center(), gpui::Modifiers::none());
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.history_view.selected.clone()),
            Some(first_entry.id.clone())
        );
        // A click selects without restoring; only the explicit action below restores.
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.input.read(cx).value().to_string()),
            "caf"
        );
        cx.simulate_keystrokes("up");
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.history_view.selected.clone()),
            Some(latest_id)
        );
        cx.simulate_keystrokes("down");
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.history_view.selected.clone()),
            Some(first_entry.id.clone())
        );

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                assert_eq!(
                    view.history_view.selected.as_deref(),
                    Some(first_entry.id.as_str())
                );
                view.sync_history(cx);
                view.restore_selected(window, cx);
                assert!(view.history_view.pending_restore.is_some());
                view.confirm_restore(window, cx);
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.mode),
            Base64Mode::Encode
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.input.read(cx).value().to_string()),
            "café"
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.result.read(cx).value().to_string()),
            "Y2Fmw6k="
        );
        assert_eq!(history.load(Base64::ID).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
