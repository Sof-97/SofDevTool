use gpui::prelude::*;
use gpui::{div, IntoElement};

use crate::theme::ThemeTokens;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// A non-color-only diagnostic banner: it always carries a text prefix and, when
/// present, a source location.
pub fn diagnostic_banner(
    severity: DiagnosticSeverity,
    message: &str,
    location: Option<(u32, u32)>,
) -> impl IntoElement {
    let tokens = ThemeTokens::active();
    let (accent, prefix) = match severity {
        DiagnosticSeverity::Error => (tokens.danger(), "Error"),
        DiagnosticSeverity::Warning => (tokens.warning(), "Warning"),
    };
    let location = location
        .map(|(line, column)| format!("line {line}, column {column}"))
        .unwrap_or_default();

    div()
        .flex()
        .flex_row()
        .items_start()
        .gap_2()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(accent)
        .bg(tokens.surface_raised())
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(accent)
                .child(format!("{prefix}:")),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_xs()
                .text_color(tokens.text())
                .child(message.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(tokens.text_muted())
                .child(location),
        )
}

/// A neutral placeholder shown when there is nothing to display.
pub fn empty_state(message: &str) -> impl IntoElement {
    let tokens = ThemeTokens::active();
    div()
        .flex()
        .items_center()
        .justify_center()
        .size_full()
        .text_sm()
        .text_color(tokens.text_muted())
        .child(message.to_string())
}

/// An unobtrusive confirmation shown after a successful copy.
pub fn copy_feedback(visible: bool, message: &str) -> impl IntoElement {
    let tokens = ThemeTokens::active();
    div()
        .text_xs()
        .text_color(tokens.accent())
        .child(if visible {
            message.to_string()
        } else {
            String::new()
        })
}
