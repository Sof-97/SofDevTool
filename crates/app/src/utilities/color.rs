//! The Color Conversion workspace: bounded CSS color input, synchronized
//! HEX/RGB/HSL representations, an interactive channel picker, an explicit
//! Clipboard surface and fresh Rust History.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, rgba, AnyView, App, Context, FocusHandle, IntoElement, Render, Subscription, Window,
};
use sofdevtool_core::session::{Session, SubmitOutcome};
use sofdevtool_core::utilities::color::{
    evaluate, outputs_for, ColorConversion, ColorEvaluation, ColorRequest, ColorSnapshot, SrgbColor,
};
use sofdevtool_core::utility::Utility;
use sofui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    ConfirmationBar, DiagnosticSeverity, NumericStepper, SelectableList, SelectableListFocus,
    SelectableRow, TextField, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder, HistorySubscription, HistoryViewState};
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

struct ButtonFocus {
    convert: FocusHandle,
    paste: FocusHandle,
    clear: FocusHandle,
    history_toggle: FocusHandle,
    history_restore: FocusHandle,
    history_confirm: FocusHandle,
    history_cancel: FocusHandle,
    channels: [FocusHandle; 8],
    copies: [FocusHandle; 3],
}

pub struct ColorWorkspace {
    source: TextField,
    clipboard: Rc<dyn Clipboard>,
    history: Rc<HistoryRecorder>,
    session: ColorSession,
    display_epoch: u64,
    copied: Option<usize>,
    suppress_changes: bool,
    history_view: HistoryViewState,
    history_visible: bool,
    history_focus: SelectableListFocus,
    _history_subscription: HistorySubscription,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl ColorWorkspace {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        clipboard: Rc<dyn Clipboard>,
        history: Rc<HistoryRecorder>,
    ) -> Self {
        let source = TextField::new(window, cx);
        let subscriptions = vec![source.on_change_in(window, cx, |this, window, cx| {
            this.schedule(window, cx);
        })];
        let history_view = HistoryViewState::load(&history, ColorConversion::ID);
        let weak = cx.weak_entity();
        let history_subscription = history.subscribe(ColorConversion::ID, move |cx| {
            weak.update(cx, |this, cx| this.reconcile_history(cx)).ok();
        });
        Self {
            source,
            clipboard,
            history,
            session: ColorSession::new(),
            display_epoch: u64::MAX,
            copied: None,
            suppress_changes: false,
            history_view,
            history_visible: true,
            history_focus: SelectableListFocus::new(),
            _history_subscription: history_subscription,
            focus: ButtonFocus {
                convert: cx.focus_handle().tab_stop(true).tab_index(0),
                paste: cx.focus_handle().tab_stop(true).tab_index(0),
                clear: cx.focus_handle().tab_stop(true).tab_index(0),
                history_toggle: cx.focus_handle().tab_stop(true).tab_index(0),
                history_restore: cx.focus_handle().tab_stop(true).tab_index(0),
                history_confirm: cx.focus_handle().tab_stop(true).tab_index(0),
                history_cancel: cx.focus_handle().tab_stop(true).tab_index(0),
                channels: std::array::from_fn(|_| cx.focus_handle().tab_stop(true).tab_index(0)),
                copies: std::array::from_fn(|_| cx.focus_handle().tab_stop(true).tab_index(0)),
            },
            _subscriptions: subscriptions,
        }
    }

    fn request(&self, cx: &App) -> ColorRequest {
        ColorRequest::new(self.source.text(cx))
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
        self.history.notify_status(cx);
        cx.notify();
    }

    fn reconcile_history(&mut self, cx: &mut Context<Self>) {
        self.history_view
            .reconcile(&self.history, ColorConversion::ID);
        cx.notify();
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

    fn adjust_channel(
        &mut self,
        channel: usize,
        delta: i32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = self.current_evaluation(cx);
        let base = match current {
            ColorEvaluation::Valid { color, .. } => color,
            ColorEvaluation::Empty => SrgbColor::new(1.0, 1.0, 1.0, 1.0),
            ColorEvaluation::Invalid { .. } => return,
        };
        let mut channels = [
            (base.red * 255.0).round() as i32,
            (base.green * 255.0).round() as i32,
            (base.blue * 255.0).round() as i32,
            (base.alpha * 255.0).round() as i32,
        ];
        channels[channel] = NumericStepper::stepped(channels[channel], delta, 0, 255);
        let next = SrgbColor::new(
            channels[0] as f64 / 255.0,
            channels[1] as f64 / 255.0,
            channels[2] as f64 / 255.0,
            channels[3] as f64 / 255.0,
        );
        let hex = outputs_for(next).hex;
        self.copied = None;
        self.source.edit_text(hex, window, cx);
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
            self.source.edit_text(text, window, cx);
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = None;
        self.source.edit_text("", window, cx);
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
        let current = self.source.text(cx);
        if !current.trim().is_empty() && current.trim() != snapshot.source.trim() {
            self.history_view.pending_restore = Some(entry);
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
        self.source.assign_text(snapshot.source.clone(), window, cx);
        self.session.restore(snapshot);
        self.display_epoch = u64::MAX;
        self.suppress_changes = false;
        self.history_view.pending_restore = None;
        self.copied = None;
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
        let tokens = ThemeTokens::active();
        div()
            .w_16()
            .h_16()
            .rounded_md()
            .border_1()
            .border_color(tokens.border())
            .bg(rgba(packed))
    }

    fn channel_row(
        &self,
        index: usize,
        label: &'static str,
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
        let weak = cx.weak_entity();
        NumericStepper::new(
            format!("color-channel-{index}"),
            label,
            (!matches!(evaluation, ColorEvaluation::Invalid { .. })).then_some(value),
            0,
            255,
            CHANNEL_STEP,
        )
        .focus_handles(
            self.focus.channels[index * 2].clone(),
            self.focus.channels[index * 2 + 1].clone(),
        )
        .on_step(move |delta, window, cx| {
            weak.update(cx, |this, cx| this.adjust_channel(index, delta, window, cx))
                .ok();
        })
    }

    fn output_row(
        &self,
        index: usize,
        label: &'static str,
        value: Option<&str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tokens = ThemeTokens::active();
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
                    .text_color(tokens.text_muted())
                    .child(label),
            )
            .child(div().flex_1().min_w_0().text_sm().child(text))
            .child(copy_feedback(self.copied == Some(index), "Copied"))
            .child(
                Button::with_id(
                    format!("color-conversion.copy.{index}"),
                    format!("Copy {label}"),
                )
                .disabled(value.is_none())
                .focus_handle(self.focus.copies[index].clone())
                .on_click(view_click(cx, move |this, _window, cx| {
                    this.copy(index, cx);
                })),
            )
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
        let actions = Button::with_id(
            "color-conversion.history.restore-selected",
            "Restore selected",
        )
        .disabled(!restore_enabled)
        .focus_handle(self.focus.history_restore.clone())
        .on_click(view_click(cx, |this, window, cx| {
            this.restore_selected(window, cx);
        }));
        let weak = cx.weak_entity();
        let list = SelectableList::new(
            "color-conversion.history",
            "History",
            self.history_items(),
            selected,
            "No retained operations yet.",
            self.history_focus.clone(),
        )
        .summary(format!("{}/25", self.history_view.entries.len()))
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
            .bg(ThemeTokens::active().surface())
            .child(list)
    }

    fn render_restore_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        ConfirmationBar::new(
            "color-conversion.history.restore",
            "Restoring this History entry replaces the current non-empty color session.",
            "Restore",
            "Cancel",
        )
        .focus_handles(
            self.focus.history_confirm.clone(),
            self.focus.history_cancel.clone(),
        )
        .on_confirm(view_click(cx, |this, window, cx| {
            this.confirm_restore(window, cx)
        }))
        .on_cancel(view_click(cx, |this, _window, cx| this.cancel_restore(cx)))
    }
}

impl Render for ColorWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::active();
        let evaluation = self.current_evaluation(cx);
        let outputs = evaluation.outputs();

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                Button::with_id("color-conversion.convert", "Convert")
                    .variant(ButtonVariant::Primary)
                    .focus_handle(self.focus.convert.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.schedule(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(copy_feedback(self.copied == Some(3), "Copied to Clipboard"))
            .child(
                Button::with_id(
                    "color-conversion.history.toggle",
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
                Button::with_id("color-conversion.paste", "Paste")
                    .focus_handle(self.focus.paste.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.paste(window, cx);
                    })),
            )
            .child(
                Button::with_id("color-conversion.clear", "Clear")
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
            .gap_3()
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Bounded 8-bit sRGB: HEX, RGB(A), HSL(A). Wide gamut is out of scope."),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .child(self.source.render("color-conversion.input")),
                    )
                    .child(self.swatch(cx)),
            )
            .child(toolbar);

        if self.history_view.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_view.error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("Color Conversion History: {error}"),
                None,
            ));
        }

        let picker = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(self.channel_row(0, "R", cx))
            .child(self.channel_row(1, "G", cx))
            .child(self.channel_row(2, "B", cx))
            .child(self.channel_row(3, "A", cx));

        let results = if matches!(evaluation, ColorEvaluation::Empty) {
            empty_state("Enter a color to begin").into_any_element()
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
            empty_state("Correct the color input to see conversions").into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_3()
            .flex_1()
            .min_h_0()
            .child(panel("Picker", "interactive sRGB channels", picker))
            .child(panel("Representations", "read-only, copyable", results));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column
            .child(workspace)
            .child(self.render_diagnostics(&evaluation))
    }
}

impl ColorWorkspace {
    fn render_diagnostics(&self, evaluation: &ColorEvaluation) -> impl IntoElement {
        let mut column = div().flex().flex_col().gap_2().w_full();
        for diagnostic in evaluation.diagnostics() {
            let severity = match diagnostic.severity {
                sofdevtool_core::diagnostic::Severity::Error => DiagnosticSeverity::Error,
                sofdevtool_core::diagnostic::Severity::Warning => DiagnosticSeverity::Warning,
            };
            column = column.child(diagnostic_banner(severity, &diagnostic.message, None));
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

    use gpui::{Entity, VisualTestContext};

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

    struct TestRoot(Entity<ColorWorkspace>);

    impl Render for TestRoot {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
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
        cx.update(sofui::init);
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
            TestRoot(view)
        });
        let workspace = captured.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.source.edit_text("#102030", window, cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 1);

        cx.update(|window, cx| {
            let focus = workspace.read(cx).focus.channels[1].clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.source.text(cx)),
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
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.copies[0].clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().last().unwrap(), "#1a2030");
        cx.update(|window, cx| {
            let focus = workspace.read(cx).focus.copies[1].clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().last().unwrap(), "rgb(26 32 48)");

        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 2);
        let latest = history.load(ColorConversion::ID).unwrap().remove(0);
        assert_eq!(decode_snapshot(&latest).unwrap().outputs.hex, "#1a2030");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let focus = workspace.read(cx).focus.channels[1].clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.source.text(cx)),
            "#1f2030"
        );
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 3);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.source.edit_text("#FE2030", window, cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.source.text(cx)),
            "#ff2030"
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let revision = workspace.read_with(&cx, |view, _| view.session.revision());
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, _| view.session.revision()),
            revision
        );
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 4);

        cx.update(|window, cx| {
            workspace.update(cx, |view, cx| {
                view.source.edit_text("invalid color", window, cx);
            });
            window.draw(cx).clear(cx);
        });
        assert!(workspace.read_with(&cx, |view, cx| view
            .current_evaluation(cx)
            .outputs()
            .is_none()));
        let writes = clipboard.0.borrow().len();
        cx.update(|window, cx| {
            let focus = workspace.read(cx).focus.copies[0].clone();
            window.focus(&focus, cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(clipboard.0.borrow().len(), writes);
        assert!(workspace.read_with(&cx, |view, _| view.copied.is_none()));
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 4);
        cx.executor().advance_clock(DEBOUNCE);
        cx.run_until_parked();
        assert_eq!(history.load(ColorConversion::ID).unwrap().len(), 4);

        let retained = history.load(ColorConversion::ID).unwrap().remove(0);
        let retained_hex = decode_snapshot(&retained).unwrap().outputs.hex;
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let list_focus = workspace.read(cx).history_focus.clone();
            window.focus(&list_focus.handle(&retained.id, cx), cx);
        });
        cx.simulate_keystrokes("enter");
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
            window.draw(cx).clear(cx);
            window.focus(&workspace.read(cx).focus.history_restore.clone(), cx);
        });
        cx.simulate_keystrokes("enter");
        assert!(workspace.read_with(&cx, |view, _| view.history_view.pending_restore.is_some()));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.focus(&workspace.read(cx).focus.history_confirm.clone(), cx);
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(
            workspace.read_with(&cx, |view, cx| view.source.text(cx)),
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
