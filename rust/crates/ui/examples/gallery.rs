//! Executable gallery for the owner-maintained component library.
//!
//! It demonstrates the exported components in their normal, focused, disabled,
//! invalid and empty states. It builds without the SofDevTool application crate.

use gpui::prelude::*;
use gpui::{
    div, App, Context, FocusHandle, IntoElement, Render, Subscription, Window, WindowOptions,
};
use sofdevtool_ui::{
    copy_feedback, diagnostic_banner, empty_state, init, mount, panel, run, set_dark_theme,
    view_click, Button, ButtonVariant, DiagnosticSeverity, LabeledField, TextEditor, TextField,
    ThemeTokens,
};

struct Gallery {
    field: TextField,
    input: TextEditor,
    result: TextEditor,
    primary_focus: FocusHandle,
    secondary_focus: FocusHandle,
    variant_focus: FocusHandle,
    copied: bool,
    _subscriptions: Vec<Subscription>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = TextField::new(window, cx);
        let input = TextEditor::new(window, cx);
        let result = TextEditor::new(window, cx);
        let subscriptions = vec![
            field.on_change_in(window, cx, |this, _window, cx| {
                this.copied = false;
                cx.notify();
            }),
            input.on_change_in(window, cx, |_this, _window, cx| {
                cx.notify();
            }),
        ];
        Self {
            field,
            input,
            result,
            primary_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            secondary_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            variant_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            copied: false,
            _subscriptions: subscriptions,
        }
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(tokens.background())
            .text_color(tokens.text())
            .p_6()
            .gap_4()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("SofDevTool component gallery"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Components are consumed from sofdevtool-ui; the gallery never imports the application."),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::primary("Primary action")
                            .focus_handle(self.primary_focus.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                this.copied = true;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("Secondary")
                            .focus_handle(self.secondary_focus.clone())
                            .on_click(view_click(cx, |_this, _window, _cx| {})),
                    )
                    .child(Button::new("Disabled").disabled(true))
                    .child(
                        Button::new("Variant")
                            .variant(ButtonVariant::Secondary)
                            .focus_handle(self.variant_focus.clone())
                            .on_click(view_click(cx, |_this, _window, _cx| {})),
                    )
                    .child(copy_feedback(self.copied, "Copied")),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Tab/Shift-Tab move focus; Enter or Space activates the focused button."),
            )
            .child(
                div()
                    .w_full()
                    .child(
                        LabeledField::new("Single-line field", self.field.render("gallery.field"))
                            .hint("normal and focused"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .flex_1()
                    .min_h_0()
                    .child(panel("Input", "multiline editor", self.input.render(false, "gallery.input")))
                    .child(panel("Result", "readonly, selectable", self.result.render(true, "gallery.result"))),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .child(diagnostic_banner(
                                DiagnosticSeverity::Error,
                                "Unexpected token at the end of the document.",
                                Some((3, 8)),
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(diagnostic_banner(
                                DiagnosticSeverity::Warning,
                                "Large input is not truncated silently.",
                                None,
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .h_24()
                    .flex_shrink_0()
                    .child(panel("Empty state", "neutral", empty_state("Nothing to show yet"))),
            )
    }
}

fn main() {
    run(|cx: &mut App| {
        init(cx);
        set_dark_theme(None, cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| Gallery::new(window, cx));
            mount(view, window, cx)
        })
        .expect("open gallery window");
    });
}
