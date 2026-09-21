//! The JSON Utility workspace: input, format/minify/query controls, diagnostics
//! and an explicit Clipboard surface.
//!
//! Input changes are debounced and revision-gated through [`JsonSession`], so an
//! obsolete asynchronous completion can never publish a result.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, App, Context, FocusHandle, IntoElement, Render, Subscription, Window};
use sofdevtool_core::json::{Indentation, JsonEvaluation, JsonMode, JsonRequest, Severity};
use sofdevtool_core::session::{JsonSession, SubmitOutcome};
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, panel, view_click, Button, ButtonVariant,
    DiagnosticSeverity, LabeledField, TextEditor, TextField, ThemeTokens,
};

use crate::clipboard::Clipboard;

const DEBOUNCE: Duration = Duration::from_millis(250);

struct ButtonFocus {
    format: FocusHandle,
    minify: FocusHandle,
    query: FocusHandle,
    indent: FocusHandle,
    sort: FocusHandle,
    paste: FocusHandle,
    copy: FocusHandle,
    clear: FocusHandle,
}

pub struct JsonWorkspace {
    input: TextEditor,
    result: TextEditor,
    query: TextField,
    clipboard: Rc<dyn Clipboard>,
    mode: JsonMode,
    indentation: Indentation,
    sort_keys: bool,
    session: JsonSession,
    display_epoch: u64,
    copied: bool,
    focus: ButtonFocus,
    _subscriptions: Vec<Subscription>,
}

impl JsonWorkspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, clipboard: Rc<dyn Clipboard>) -> Self {
        let input = TextEditor::new(window, cx);
        let result = TextEditor::new(window, cx);
        let query = TextField::new(window, cx);

        let subscriptions = vec![
            input.on_change_in(window, cx, |this, window, cx| {
                this.schedule(window, cx);
            }),
            query.on_change_in(window, cx, |this, window, cx| {
                this.schedule(window, cx);
            }),
        ];

        Self {
            input,
            result,
            query,
            clipboard,
            mode: JsonMode::Format,
            indentation: Indentation::TwoSpaces,
            sort_keys: false,
            session: JsonSession::new(),
            display_epoch: u64::MAX,
            copied: false,
            focus: ButtonFocus {
                format: cx.focus_handle().tab_stop(true).tab_index(0),
                minify: cx.focus_handle().tab_stop(true).tab_index(0),
                query: cx.focus_handle().tab_stop(true).tab_index(0),
                indent: cx.focus_handle().tab_stop(true).tab_index(0),
                sort: cx.focus_handle().tab_stop(true).tab_index(0),
                paste: cx.focus_handle().tab_stop(true).tab_index(0),
                copy: cx.focus_handle().tab_stop(true).tab_index(0),
                clear: cx.focus_handle().tab_stop(true).tab_index(0),
            },
            _subscriptions: subscriptions,
        }
    }

    fn request(&self, cx: &App) -> JsonRequest {
        JsonRequest {
            input: self.input.text(cx),
            mode: self.mode,
            indentation: self.indentation,
            sort_keys: self.sort_keys,
            query: self.query.text(cx),
        }
    }

    /// Submits the current request. A changed request clears the visible result
    /// immediately and schedules a debounced, revision-gated evaluation.
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let request = self.request(cx);
        let SubmitOutcome::Scheduled(revision) = self.session.submit(request) else {
            return;
        };
        self.sync_display(window, cx);
        cx.notify();

        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            executor.timer(DEBOUNCE).await;
            this.update(cx, |this, cx| {
                if this.session.resolve(revision).is_some() {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// Mirrors the session's settled evaluation into the result editor, but only
    /// when the evaluation epoch changed. Obsolete results are never shown.
    fn sync_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.session.evaluation_epoch();
        if self.display_epoch == epoch {
            return;
        }
        self.display_epoch = epoch;
        match self.session.evaluation() {
            JsonEvaluation::Valid { output } => {
                self.result.set_text(output.clone(), window, cx);
            }
            _ => self.result.set_text("", window, cx),
        }
    }

    fn set_mode(&mut self, mode: JsonMode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        self.schedule(window, cx);
    }

    fn toggle_indentation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.indentation = match self.indentation {
            Indentation::TwoSpaces => Indentation::FourSpaces,
            Indentation::FourSpaces => Indentation::TwoSpaces,
        };
        self.schedule(window, cx);
    }

    fn toggle_sort(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sort_keys = !self.sort_keys;
        self.schedule(window, cx);
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard.read_text(cx) {
            self.copied = false;
            // replace_all records undo history and emits a change event.
            self.input.replace_all(text, window, cx);
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
        self.input.replace_all("", window, cx);
    }

    fn mode_button(&self, label: &'static str, mode: JsonMode, cx: &mut Context<Self>) -> Button {
        Button::new(label)
            .variant(if self.mode == mode {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .focus_handle(match mode {
                JsonMode::Format => self.focus.format.clone(),
                JsonMode::Minify => self.focus.minify.clone(),
                JsonMode::Query => self.focus.query.clone(),
            })
            .on_click(view_click(cx, move |this, window, cx| {
                this.set_mode(mode, window, cx);
            }))
    }

    fn render_diagnostics(&self) -> impl IntoElement {
        let mut column = div().flex().flex_col().gap_2().w_full();
        for diagnostic in self.session.evaluation().diagnostics() {
            let severity = match diagnostic.severity {
                Severity::Error => DiagnosticSeverity::Error,
                Severity::Warning => DiagnosticSeverity::Warning,
            };
            let location = diagnostic.location.map(|l| (l.line, l.column));
            column = column.child(diagnostic_banner(severity, &diagnostic.message, location));
        }
        column
    }
}

impl Render for JsonWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_display(window, cx);

        let tokens = ThemeTokens::graphite();
        let can_copy = self.session.evaluation().is_valid_operation();
        let pending = matches!(self.session.evaluation(), JsonEvaluation::Empty)
            && self
                .session
                .request()
                .map(|request| !request.input.trim().is_empty())
                .unwrap_or(false);

        let mut toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(self.mode_button("Format", JsonMode::Format, cx))
            .child(self.mode_button("Minify", JsonMode::Minify, cx))
            .child(self.mode_button("Query", JsonMode::Query, cx));

        if self.mode == JsonMode::Format {
            toolbar = toolbar.child(
                Button::new(match self.indentation {
                    Indentation::TwoSpaces => "Indent: 2",
                    Indentation::FourSpaces => "Indent: 4",
                })
                .focus_handle(self.focus.indent.clone())
                .on_click(view_click(cx, |this, window, cx| {
                    this.toggle_indentation(window, cx);
                })),
            );
        }

        toolbar = toolbar
            .child(
                Button::new(if self.sort_keys {
                    "Sort keys: on"
                } else {
                    "Sort keys: off"
                })
                .variant(if self.sort_keys {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                })
                .focus_handle(self.focus.sort.clone())
                .on_click(view_click(cx, |this, window, cx| {
                    this.toggle_sort(window, cx);
                })),
            )
            .child(div().flex_1())
            .child(copy_feedback(self.copied, "Copied to Clipboard"))
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
                            .child("JSON"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child("Local, offline, unsorted by default"),
                    ),
            )
            .child(toolbar);

        if self.mode == JsonMode::Query {
            column = column.child(
                LabeledField::new("Path", self.query.render("json.query"))
                    .hint("JSON Pointer (/a/b) or dot/bracket (a.b[0])"),
            );
        }

        let result_body = if pending {
            empty_state("Evaluating…").into_any_element()
        } else if matches!(self.session.evaluation(), JsonEvaluation::Empty) {
            empty_state("Paste JSON to begin").into_any_element()
        } else {
            self.result.render(true, "json.result").into_any_element()
        };

        column
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .flex_1()
                    .min_h_0()
                    .child(panel(
                        "Input",
                        "live validation",
                        self.input.render(false, "json.input"),
                    ))
                    .child(panel("Result", "read-only, selectable", result_body)),
            )
            .child(self.render_diagnostics())
    }
}
