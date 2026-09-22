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
    outputs_for, ColorConversion, ColorEvaluation, ColorOutputs, ColorRequest, ColorSnapshot,
    SrgbColor,
};
use sofdevtool_core::utility::Utility;
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    DiagnosticSeverity, HistoryItem, HistoryPanel, TextField, ThemeTokens,
};

use crate::clipboard::Clipboard;
use crate::history::{HistoryEntry, HistoryRecorder};
use crate::workbench::Workbench;

const DEBOUNCE: Duration = Duration::from_millis(200);
const CHANNEL_STEP: i16 = 5;

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
    history_entries: Vec<HistoryEntry>,
    history_selected: Option<String>,
    history_visible: bool,
    history_error: Option<String>,
    pending_restore: Option<HistoryEntry>,
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
        let (history_entries, history_error) = match history.load(ColorConversion::ID) {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        Self {
            source,
            clipboard,
            history,
            session: ColorSession::new(),
            display_epoch: u64::MAX,
            copied: None,
            suppress_changes: false,
            history_entries,
            history_selected: None,
            history_visible: true,
            history_error,
            pending_restore: None,
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
        match self.history.record(
            ColorConversion::ID,
            ColorConversion::SNAPSHOT_VERSION,
            payload,
        ) {
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

    /// The color display never needs a separate editor: the channel picker reads
    /// the session's settled evaluation directly.
    fn sync_display(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        self.copied = None;
    }

    fn current_color(&self) -> Option<SrgbColor> {
        self.session.evaluation().color()
    }

    fn adjust_channel(
        &mut self,
        channel: usize,
        delta: i16,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let base = self
            .current_color()
            .unwrap_or(SrgbColor::new(1.0, 1.0, 1.0, 1.0));
        let mut channels = [
            (base.red * 255.0).round() as i16,
            (base.green * 255.0).round() as i16,
            (base.blue * 255.0).round() as i16,
            (base.alpha * 255.0).round() as i16,
        ];
        channels[channel] = (channels[channel] + delta).clamp(0, 255);
        let next = SrgbColor::new(
            channels[0] as f64 / 255.0,
            channels[1] as f64 / 255.0,
            channels[2] as f64 / 255.0,
            channels[3] as f64 / 255.0,
        );
        let hex = outputs_for(next).hex;
        self.copied = None;
        self.source.set_text(hex, window, cx);
    }

    fn copy(&mut self, index: usize, value: &str, cx: &mut Context<Self>) {
        self.clipboard.write_text(value, cx);
        self.copied = Some(index);
        cx.notify();
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = None;
            self.source.replace_all(text, window, cx);
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copied = None;
        self.source.replace_all("", window, cx);
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
        let current = self.source.text(cx);
        if !current.trim().is_empty() && current.trim() != snapshot.source.trim() {
            self.pending_restore = Some(entry);
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
        self.source.set_text(snapshot.source.clone(), window, cx);
        self.session.restore(snapshot);
        self.display_epoch = u64::MAX;
        self.suppress_changes = false;
        self.pending_restore = None;
        self.copied = None;
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
                        .map(|snapshot| format!("{} · {}", snapshot.outputs.hex, snapshot.source))
                        .unwrap_or_else(|| "Unavailable snapshot".to_owned()),
                    available: snapshot.is_some(),
                }
            })
            .collect()
    }

    fn swatch(&self) -> impl IntoElement {
        let color = self
            .current_color()
            .unwrap_or(SrgbColor::new(0.0, 0.0, 0.0, 1.0));
        let channel = |value: f64| (value * 255.0).round().clamp(0.0, 255.0) as u32;
        let packed = (channel(color.red) << 24)
            | (channel(color.green) << 16)
            | (channel(color.blue) << 8)
            | channel(color.alpha);
        let tokens = ThemeTokens::graphite();
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
        let color = self.current_color();
        let value = match (index, color) {
            (0, Some(color)) => (color.red * 255.0).round() as i32,
            (1, Some(color)) => (color.green * 255.0).round() as i32,
            (2, Some(color)) => (color.blue * 255.0).round() as i32,
            (3, Some(color)) => (color.alpha * 255.0).round() as i32,
            _ => 255,
        };
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(div().w_6().text_xs().child(label))
            .child(
                Button::new("−")
                    .id(gpui::ElementId::Name(
                        format!("color-channel-{index}-minus").into(),
                    ))
                    .focus_handle(self.focus.channels[index * 2].clone())
                    .on_click(view_click(cx, move |this, window, cx| {
                        this.adjust_channel(index, -CHANNEL_STEP, window, cx);
                    })),
            )
            .child(
                div()
                    .w_10()
                    .text_xs()
                    .text_color(ThemeTokens::graphite().text_muted())
                    .child(format!("{value}")),
            )
            .child(
                Button::new("+")
                    .id(gpui::ElementId::Name(
                        format!("color-channel-{index}-plus").into(),
                    ))
                    .focus_handle(self.focus.channels[index * 2 + 1].clone())
                    .on_click(view_click(cx, move |this, window, cx| {
                        this.adjust_channel(index, CHANNEL_STEP, window, cx);
                    })),
            )
    }

    fn output_row(
        &self,
        index: usize,
        label: &'static str,
        value: Option<&str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
        let text = value.unwrap_or("").to_owned();
        let copy_value = text.clone();
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
                Button::new(format!("Copy {label}"))
                    .disabled(value.is_none())
                    .focus_handle(self.focus.copies[index].clone())
                    .on_click(view_click(cx, move |this, _window, cx| {
                        this.copy(index, &copy_value, cx);
                    })),
            )
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
        let panel = HistoryPanel::new(
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
            .border_color(ThemeTokens::graphite().border())
            .child(panel)
    }

    fn render_restore_confirmation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
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
                div()
                    .text_xs()
                    .text_color(tokens.text())
                    .child("Restoring this History entry replaces the current color session."),
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

impl Render for ColorWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);
        let tokens = ThemeTokens::graphite();
        let outputs: Option<ColorOutputs> = self.session.evaluation().outputs().cloned();
        let pending = matches!(self.session.evaluation(), ColorEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.source.trim().is_empty())
                .unwrap_or(false);

        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                Button::new("Convert")
                    .variant(ButtonVariant::Primary)
                    .focus_handle(self.focus.convert.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        this.schedule(window, cx);
                    })),
            )
            .child(div().flex_1())
            .child(copy_feedback(self.copied == Some(3), "Copied to Clipboard"))
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
                            .child("Color Conversion"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Bounded sRGB: HEX, RGB(A), HSL(A)"),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Values normalize to 8-bit sRGB; wide gamut is out of scope."),
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
                    .child(self.swatch()),
            )
            .child(toolbar);

        if self.pending_restore.is_some() {
            column = column.child(self.render_restore_confirmation(cx));
        }
        if let Some(error) = self.history_error.clone() {
            column = column.child(diagnostic_banner(
                DiagnosticSeverity::Warning,
                &format!("History is paused for Color Conversion: {error}"),
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

        let results = if pending {
            empty_state("Converting…").into_any_element()
        } else if matches!(self.session.evaluation(), ColorEvaluation::Empty) {
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
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(self.render_diagnostics())
                .into_any_element()
        };

        let mut workspace = div()
            .flex()
            .flex_row()
            .gap_4()
            .flex_1()
            .min_h_0()
            .child(panel("Picker", "interactive sRGB channels", picker))
            .child(panel("Representations", "read-only, copyable", results));
        if self.history_visible {
            workspace = workspace.child(self.render_history(cx));
        }
        column.child(workspace).child(self.render_diagnostics())
    }
}

impl ColorWorkspace {
    fn render_diagnostics(&self) -> impl IntoElement {
        let mut column = div().flex().flex_col().gap_2().w_full();
        for diagnostic in self.session.evaluation().diagnostics() {
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
