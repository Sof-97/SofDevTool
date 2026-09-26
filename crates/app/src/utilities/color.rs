//! The Color Conversion workspace: bounded CSS color input, synchronized
//! HEX/RGB/HSL representations, an interactive channel picker, an explicit
//! Clipboard surface and fresh Rust History.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, rgba, AnyView, App, Context, Entity, IntoElement, Render, Subscription, Window};
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState, NumberInput},
    list::ListState,
    ActiveTheme as _, Disableable as _,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::color::{
    evaluate, outputs_for, ColorConversion, ColorEvaluation, ColorRequest, ColorSnapshot, SrgbColor,
};
use sofdevtool_core::utility::Utility;

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
use crate::ui;
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(200);
const CHANNEL_STEP: i32 = 5;

type ColorSession = Session<ColorConversion>;

/// Builds the Color Conversion workspace as a type-erased view.
pub fn construct(
    window: &mut Window,
    cx: &mut Context<Workbench>,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
) -> AnyView {
    cx.new(|cx| ColorWorkspace::new(window, cx, clipboard, history))
        .into()
}

pub struct ColorWorkspace {
    source: Entity<InputState>,
    channels: [Entity<InputState>; 4],
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    session: ColorSession,
    display_epoch: u64,
    copied: Option<usize>,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    history_list: Entity<ListState<ui::HistoryListDelegate>>,
    _history_subscription: HistorySubscription,
    _subscriptions: Vec<Subscription>,
}

impl ColorWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let source = cx.new(|cx| InputState::new(window, cx));
        let channels: [Entity<InputState>; 4] = std::array::from_fn(|_| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value("255")
                    .min(0.0)
                    .max(255.0)
                    .step_by(|_, _action, _| CHANNEL_STEP as f64)
            })
        });
        let mut subscriptions = vec![cx.subscribe_in(
            &source,
            window,
            |this, _entity, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.schedule(window, cx);
                }
            },
        )];
        for (index, state) in channels.iter().enumerate() {
            subscriptions.push(cx.subscribe_in(
                state,
                window,
                move |this, _entity, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.set_channel(index, window, cx);
                    }
                },
            ));
        }
        let history_view = HistoryViewState::load(&history, ColorConversion::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(ColorConversion::ID, move |cx| {
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
            source,
            channels,
            clipboard,
            history,
            session: ColorSession::new(),
            display_epoch: u64::MAX,
            copied: None,
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

    fn request(&self, cx: &App) -> ColorRequest {
        ColorRequest::new(self.source.read(cx).value().to_string())
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
        let payload = serde_json::to_value(&snapshot).expect("a Color snapshot serializes");
        let result = self.history.record(
            ColorConversion::ID,
            ColorConversion::SNAPSHOT_VERSION,
            payload,
        );
        self.history_view
            .apply_record(&self.history, ColorConversion::ID, result);
        self.sync_history(cx);
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view
            .reconcile(&self.history, ColorConversion::ID);
        self.sync_history(cx);
        cx.notify();
    }

    /// Reflects the owning view's History rows and selection into the kit list.
    fn sync_history(&self, cx: &mut Context<Self>) {
        ui::history_set_rows(&self.history_list, self.history_items(), cx);
        ui::history_set_selected(&self.history_list, self.history_view.selected.clone(), cx);
    }

    /// The visible result follows current editor text, while the session still
    /// gates settlement and History to the final requested revision.
    fn sync_display(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        self.copied = None;
    }

    fn current_evaluation(&self, cx: &App) -> ColorEvaluation {
        evaluate(&self.request(cx))
    }

    fn current_color(&self, cx: &App) -> Option<SrgbColor> {
        self.current_evaluation(cx).color()
    }

    /// Applies the channel editor's latest value to the current colour.
    ///
    /// GPUI Kit's number input owns stepping; this reads the resulting value
    /// after the change, so it always applies to the most recent base colour.
    fn set_channel(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.suppress_changes {
            return;
        }
        let base = match self.current_evaluation(cx) {
            ColorEvaluation::Valid { color, .. } => color,
            ColorEvaluation::Empty => SrgbColor::new(1.0, 1.0, 1.0, 1.0),
            ColorEvaluation::Invalid { .. } => return,
        };
        let Ok(value) = self.channels[index].read(cx).value().trim().parse::<i32>() else {
            return;
        };
        let mut channels = [
            (base.red * 255.0).round() as i32,
            (base.green * 255.0).round() as i32,
            (base.blue * 255.0).round() as i32,
            (base.alpha * 255.0).round() as i32,
        ];
        channels[index] = value.clamp(0, 255);
        let next = SrgbColor::new(
            channels[0] as f64 / 255.0,
            channels[1] as f64 / 255.0,
            channels[2] as f64 / 255.0,
            channels[3] as f64 / 255.0,
        );
        let hex = outputs_for(next).hex;
        self.copied = None;
        // `set_value` does not emit; schedule explicitly from this change so the
        // channel and source subscriptions never re-enter each other.
        self.source
            .update(cx, |state, cx| state.set_value(hex, window, cx));
        self.schedule(window, cx);
    }

    fn copy(&mut self, index: usize, cx: &mut Context<Self>) {
        let current = self.current_evaluation(cx);
        let Some(outputs) = current.outputs() else {
            return;
        };
        let value = match index {
            0 => &outputs.hex,
            1 => &outputs.rgb,
            2 => &outputs.hsl,
            _ => return,
        };
        self.clipboard.write_text(value, cx);
        self.copied = Some(index);
        cx.notify();
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = None;
            self.source
                .update(cx, |state, cx| state.replace_all(text, window, cx));
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = None;
        self.source
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
        let current = self.source.read(cx).value().to_string();
        if !current.trim().is_empty() && current.trim() != snapshot.source.trim() {
            self.history_view.pending_restore = Some(entry);
            let weak = cx.weak_entity();
            ui::confirm_dialog(
                window,
                cx,
                "Restore History entry",
                "Restoring this History entry replaces the current non-empty color session.",
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
        snapshot: ColorSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.suppress_changes = true;
        self.source.update(cx, |state, cx| {
            state.set_value(snapshot.source.clone(), window, cx)
        });
        self.session.restore(snapshot);
        self.display_epoch = u64::MAX;
        self.suppress_changes = false;
        self.history_view.pending_restore = None;
        self.copied = None;
        self.sync_history(cx);
        cx.notify();
    }

    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.history_view.pending_restore.clone() {
            if self
                .history_view
                .retained(&self.history, ColorConversion::ID, &entry)
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
                .retained(&self.history, ColorConversion::ID, &entry)
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
                        .map(|snapshot| format!("{} · {}", snapshot.outputs.hex, snapshot.source))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    status: snapshot.is_none().then(|| "Unavailable".to_owned()),
                    selectable: true,
                }
            })
            .collect()
    }

    fn swatch(&self, cx: &App) -> impl IntoElement {
        let color = self
            .current_color(cx)
            .unwrap_or(SrgbColor::new(0.0, 0.0, 0.0, 1.0));
        let channel = |value: f64| (value * 255.0).round().clamp(0.0, 255.0) as u32;
        let packed = (channel(color.red) << 24)
            | (channel(color.green) << 16)
            | (channel(color.blue) << 8)
            | channel(color.alpha);
        let theme = cx.theme().clone();
        div()
            .w_16()
            .h_16()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(rgba(packed))
    }

    fn channel_row(
        &self,
        index: usize,
        label: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let evaluation = self.current_evaluation(cx);
        let color = evaluation.color();
        let value = match (index, color) {
            (0, Some(color)) => (color.red * 255.0).round() as i32,
            (1, Some(color)) => (color.green * 255.0).round() as i32,
            (2, Some(color)) => (color.blue * 255.0).round() as i32,
            (3, Some(color)) => (color.alpha * 255.0).round() as i32,
            (_, None) if matches!(evaluation, ColorEvaluation::Empty) => 255,
            _ => 0,
        };
        let disabled = matches!(evaluation, ColorEvaluation::Invalid { .. });
        // Keep the retained kit number input showing the current colour.
        // Programmatic `set_value` emits no change, so this cannot loop.
        self.channels[index].update(cx, |state, cx| {
            let text = value.to_string();
            if state.value().as_ref() != text.as_str() {
                state.set_value(text, window, cx);
            }
        });
        let theme = cx.theme().clone();
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .w_full()
            .child(
                div()
                    .w_6()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(label),
            )
            .child(
                div()
                    .w_32()
                    .child(NumberInput::new(&self.channels[index]).disabled(disabled)),
            )
    }

    fn output_row(
        &self,
        index: usize,
        label: &'static str,
        value: Option<&str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme().clone();
        let text = value.unwrap_or("").to_owned();
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .w_full()
            .child(
                div()
                    .w_10()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(label),
            )
            .child(div().flex_1().min_w_0().text_sm().child(text))
            .child(ui::copy_feedback(cx, self.copied == Some(index), "Copied"))
            .child(
                Button::new(format!("color-conversion.copy.{index}"))
                    .label(format!("Copy {label}"))
                    .disabled(value.is_none())
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.copy(index, cx);
                    })),
            )
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
            .is_some_and(|entry| decode_snapshot(entry).is_some());
        let restore = Button::new("color-conversion.history.restore-selected")
            .label("Restore selected")
            .disabled(!restore_enabled)
            .on_click(cx.listener(|this, _event, window, cx| {
                this.restore_selected(window, cx);
            }));
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

impl Render for ColorWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let theme = cx.theme().clone();
        let evaluation = self.current_evaluation(cx);
        let outputs = evaluation.outputs();

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                Button::new("color-conversion.convert")
                    .label("Convert")
                    .primary()
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.schedule(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(ui::copy_feedback(
                cx,
                self.copied == Some(3),
                "Copied to Clipboard",
            ))
            .child(
                Button::new("color-conversion.history.toggle")
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
                Button::new("color-conversion.paste")
                    .label("Paste")
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::new("color-conversion.clear")
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
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Bounded 8-bit sRGB: HEX, RGB(A), HSL(A). Wide gamut is out of scope."),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(
                        div().flex_1().child(
                            Input::new(&self.source)
                                .accessibility_id("color-conversion.input")
                                .w_full(),
                        ),
                    )
                    .child(self.swatch(cx)),
            )
            .child(toolbar);

        if let Some(error) = self.history_view.error.clone() {
            column = column.child(ui::diagnostic_banner(
                cx,
                ui::DiagnosticSeverity::Warning,
                &format!("Color Conversion History: {error}"),
                None,
            ));
        }

        let picker = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.channel_row(0, "R", window, cx))
            .child(self.channel_row(1, "G", window, cx))
            .child(self.channel_row(2, "B", window, cx))
            .child(self.channel_row(3, "A", window, cx));

        let results = if matches!(evaluation, ColorEvaluation::Empty) {
            ui::empty_state(cx, "Enter a color to begin").into_any_element()
        } else if let Some(outputs) = &outputs {
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(self.output_row(0, "HEX", Some(&outputs.hex), cx))
                .child(self.output_row(1, "RGB", Some(&outputs.rgb), cx))
                .child(self.output_row(2, "HSL", Some(&outputs.hsl), cx))
                .into_any_element()
        } else {
            ui::empty_state(cx, "Correct the color input to see conversions").into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(ui::panel(cx, "Picker", "interactive sRGB channels", picker))
            .child(ui::panel(
                cx,
                "Representations",
                "read-only, copyable",
                results,
            ));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column
            .child(workspace)
            .child(self.render_diagnostics(&evaluation, cx))
    }
}

impl ColorWorkspace {
    fn render_diagnostics(&self, evaluation: &ColorEvaluation, cx: &App) -> impl IntoElement {
        let mut column = div().flex().flex_col().gap_2().w_full();
        for diagnostic in evaluation.diagnostics() {
            let severity = match diagnostic.severity {
                sofdevtool_core::diagnostic::Severity::Error => ui::DiagnosticSeverity::Error,
                sofdevtool_core::diagnostic::Severity::Warning => ui::DiagnosticSeverity::Warning,
            };
            column = column.child(ui::diagnostic_banner(
                cx,
                severity,
                &diagnostic.message,
                None,
            ));
        }
        column
    }
}

fn decode_snapshot(entry: &HistoryEntry) -> Option<ColorSnapshot> {
    if entry.snapshot_version != ColorConversion::SNAPSHOT_VERSION {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;

    use gpui::{Focusable as _, VisualTestContext};

    use crate::history::{HistoryStore, SystemClock};
    use sofdevtool_core::utilities::color::parse;

    #[derive(Default)]
    struct TestClipboard(RefCell<Vec<String>>);

    impl Clipboard for TestClipboard {
        fn read_text(&self, _cx: &mut App) -> Option<String> {
            None
        }

        fn write_text(&self, text: &str, _cx: &mut App) {
            self.0.borrow_mut().push(text.to_owned());
        }
    }

    fn isolated_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-color-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[gpui::test]
    fn picker_events_accumulate_latest_color_and_record_only_settled_revision(
        cx: &mut gpui::TestAppContext,
    ) {
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
                cx.new(|cx| ColorWorkspace::new(window, cx, clipboard.clone(), history.clone()));
            captured = Some(view.clone());
            gpui_kit::component::Root::new(view, window, cx)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.source
                    .update(cx, |state, cx| state.replace_all("#102030", window, cx));
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 1);

        // Focus the R channel's kit number input and step it up twice.
        cx.update(|window, cx| {
            let focus = workspace.read(cx).channels[0].read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("up");
        cx.simulate_keystrokes("up");
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.source.read(cx).value().to_string()),
            "#1a2030"
        );
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.session.revision()),
            3
        );
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 1);
        let visible = workspace.read_with(&cx, |view, cx| view.current_evaluation(cx));
        let visible_color = visible.color().unwrap();
        let visible_outputs = visible.outputs().unwrap();
        assert_eq!(visible_outputs.rgb, "rgb(26 32 48)");
        assert_eq!(parse(&visible_outputs.hsl).unwrap(), visible_color);
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.current_color(cx)),
            Some(visible_color)
        );
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy(0, cx));
        });
        assert_eq!(clipboard.0.borrow().last().unwrap(), "#1a2030");
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy(1, cx));
        });
        assert_eq!(clipboard.0.borrow().last().unwrap(), "rgb(26 32 48)");

        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 2);
        let latest = history.load(ColorConversion::ID).unwrap().remove(0);
        assert_eq!(decode_snapshot(&latest).unwrap().outputs.hex, "#1a2030");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).channels[0].read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("up");
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.source.read(cx).value().to_string()),
            "#1f2030"
        );
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 3);

        // Type a new colour, then step the still-focused channel to its maximum.
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.source
                    .update(cx, |state, cx| state.replace_all("#FE2030", window, cx));
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("up");
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.source.read(cx).value().to_string()),
            "#ff2030"
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let revision = workspace.read_with(&cx, |view, _| view.session.revision());
        cx.simulate_keystrokes("up");
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.session.revision()),
            revision
        );
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 4);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.source.update(cx, |state, cx| {
                    state.replace_all("invalid color", window, cx)
                });
            });
            window.draw(cx).clear(cx);
        });
        assert!(workspace.read_with(&cx, |view, cx| view
            .current_evaluation(cx)
            .outputs()
            .is_none()));
        let writes = clipboard.0.borrow().len();
        // The kit Button is disabled for an invalid colour; `copy` is the same
        // domain no-op its activation would perform.
        cx.update(|_window, cx| {
            workspace.update(cx, |view, cx| view.copy(0, cx));
        });
        assert_eq!(clipboard.0.borrow().len(), writes);
        assert!(workspace.read_with(&cx, |view, _| view.copied.is_none()));
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 4);
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 4);

        let retained = history.load(ColorConversion::ID).unwrap().remove(0);
        let retained_hex = decode_snapshot(&retained).unwrap().outputs.hex;
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                assert!(view.history_view.select(&retained.id));
                view.sync_history(cx);
            });
            window.draw(cx).clear(cx);
        });
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.history_view.selected.clone()),
            Some(retained.id.clone())
        );
        assert!(workspace.read_with(&cx, |view, _| {
            view.history_view
                .entries
                .iter()
                .find(|entry| entry.id == retained.id)
                .is_some_and(|entry| {
                    view.history_view
                        .retained(&history, ColorConversion::ID, entry)
                })
        }));
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.restore_selected(window, cx);
            });
        });
        assert!(workspace.read_with(&cx, |view, _| view.history_view.pending_restore.is_some()));
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.confirm_restore(window, cx);
            });
        });
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.source.read(cx).value().to_string()),
            retained_hex
        );
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view
                .current_evaluation(cx)
                .outputs()
                .unwrap()
                .hex
                .clone()),
            retained_hex
        );
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 4);
        fs::remove_dir_all(root).unwrap();
    }
}
