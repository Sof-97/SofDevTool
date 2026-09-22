use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, AnyElement, App, ElementId, IntoElement, RenderOnce, SharedString, Window};

use crate::theme::ThemeTokens;

/// One retained History entry, reduced to what a list row needs to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryItem {
    pub id: String,
    /// Short, human label such as a captured timestamp.
    pub label: String,
    /// A short, single-line preview of the retained payload.
    pub preview: String,
    /// False when the stored snapshot version is unknown to this build.
    pub available: bool,
}

/// A selection handler. The entry id is passed by reference; the handler is
/// owned by the host view.
pub type HistoryAction = Rc<dyn Fn(&str, &mut Window, &mut App) + 'static>;

/// A reusable, scrollable History list with a caller-supplied action row.
///
/// The component owns no Utility identity or persistence: callers pass plain
/// items, a selected id, and (optionally) an action row built from their own
/// focus-managed buttons.
#[derive(IntoElement)]
pub struct HistoryPanel {
    items: Vec<HistoryItem>,
    selected: Option<String>,
    empty_message: SharedString,
    on_select: Option<HistoryAction>,
    actions: Option<AnyElement>,
}

impl HistoryPanel {
    pub fn new(
        items: Vec<HistoryItem>,
        selected: Option<String>,
        empty_message: impl Into<SharedString>,
    ) -> Self {
        Self {
            items,
            selected,
            empty_message: empty_message.into(),
            on_select: None,
            actions: None,
        }
    }

    pub fn on_select(mut self, handler: HistoryAction) -> Self {
        self.on_select = Some(handler);
        self
    }

    /// A row of controls rendered under the list, usually disabled until an
    /// entry is selected.
    pub fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }
}

impl RenderOnce for HistoryPanel {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let count = self.items.len();
        let mut list = div()
            .id("history-list")
            .role(gpui::Role::ListBox)
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .overflow_y_scroll();
        for item in &self.items {
            let selected = self.selected.as_deref() == Some(item.id.as_str());
            let mut row = div()
                .id(ElementId::Name(SharedString::from(format!(
                    "history-row-{}",
                    item.id
                ))))
                .role(gpui::Role::ListBoxOption)
                .aria_selected(selected)
                .aria_label(SharedString::from(format!(
                    "{} {}",
                    item.label, item.preview
                )))
                .flex()
                .flex_col()
                .gap_1()
                .w_full()
                .px_2()
                .py_2()
                .rounded_md()
                .border_1()
                .border_color(if selected {
                    tokens.accent()
                } else {
                    tokens.border()
                })
                .bg(if selected {
                    tokens.surface_raised()
                } else {
                    tokens.surface()
                })
                .cursor_pointer()
                .hover(|style| style.bg(tokens.surface_raised()))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(tokens.text_muted())
                                .child(item.label.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(if item.available {
                                    tokens.text_muted()
                                } else {
                                    tokens.warning()
                                })
                                .child(if item.available {
                                    String::new()
                                } else {
                                    "Unavailable snapshot".to_owned()
                                }),
                        ),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(tokens.text())
                        .child(item.preview.clone()),
                );
            if let Some(on_select) = self.on_select.clone() {
                let id = item.id.clone();
                row = row.on_click(move |_event, window, cx| on_select(&id, window, cx));
            }
            list = list.child(row);
        }

        let body = if count == 0 {
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(tokens.text_muted())
                .child(self.empty_message)
                .into_any_element()
        } else {
            list.into_any_element()
        };

        let mut panel = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(tokens.text())
                            .child("History"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(tokens.text_muted())
                            .child(format!("{count}/25")),
                    ),
            )
            .child(body);
        if let Some(actions) = self.actions {
            panel = panel.child(actions);
        }
        panel
    }
}
