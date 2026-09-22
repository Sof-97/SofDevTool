use gpui::prelude::*;
use gpui::{div, App, IntoElement, RenderOnce, SharedString, Window};

use crate::theme::ThemeTokens;

/// A labelled region with a muted caption, used to frame editors and results.
pub fn panel(
    title: impl Into<SharedString>,
    caption: impl Into<SharedString>,
    content: impl IntoElement,
) -> impl IntoElement {
    let tokens = ThemeTokens::active();
    let title = title.into();
    let caption = caption.into();
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .rounded_md()
        .border_1()
        .border_color(tokens.border())
        .bg(tokens.surface())
        .overflow_hidden()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(tokens.border())
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(tokens.text())
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(tokens.text_muted())
                        .child(caption),
                ),
        )
        // This wrapper establishes the vertical flex context that gives
        // editors and the native WebView their remaining panel height.
        .child(div().flex().flex_col().flex_1().min_h_0().child(content))
}

/// A label stacked above a control.
#[derive(IntoElement)]
pub struct LabeledField {
    label: SharedString,
    hint: Option<SharedString>,
    control: gpui::AnyElement,
}

impl LabeledField {
    pub fn new(label: impl Into<SharedString>, control: impl IntoElement) -> Self {
        Self {
            label: label.into(),
            hint: None,
            control: control.into_any_element(),
        }
    }

    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl RenderOnce for LabeledField {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let mut header = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .w_full()
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child(self.label),
            );
        if let Some(hint) = self.hint {
            header = header.child(div().text_xs().text_color(tokens.text_muted()).child(hint));
        }
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .min_w_0()
            .child(header)
            .child(div().flex().flex_1().min_h_0().child(self.control))
    }
}
