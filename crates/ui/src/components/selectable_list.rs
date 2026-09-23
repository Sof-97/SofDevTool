use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, AnyElement, App, ElementId, FocusHandle, IntoElement, KeyBinding, RenderOnce,
    SharedString, Window,
};

use crate::theme::ThemeTokens;

gpui::actions!(sofui_selectable_list, [ActivateRow]);

pub const SELECTABLE_LIST_KEY_CONTEXT: &str = "SofuiSelectableList";

pub(crate) fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", ActivateRow, Some(SELECTABLE_LIST_KEY_CONTEXT)),
        KeyBinding::new("space", ActivateRow, Some(SELECTABLE_LIST_KEY_CONTEXT)),
    ]);
}

/// Application-supplied content for one selectable row. `status` and
/// `selectable` are presentation only; sofui assigns no product meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectableRow {
    pub id: String,
    pub label: String,
    pub preview: String,
    pub status: Option<String>,
    pub selectable: bool,
}

pub type SelectAction = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// Retains per-row focus identity across redraws and reordered content.
#[derive(Clone, Default)]
pub struct SelectableListFocus(Rc<RefCell<BTreeMap<String, FocusHandle>>>);

impl SelectableListFocus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle(&self, id: &str, cx: &mut App) -> FocusHandle {
        self.0
            .borrow_mut()
            .entry(id.to_owned())
            .or_insert_with(|| cx.focus_handle().tab_stop(true).tab_index(0))
            .clone()
    }
}

/// Generic selection, focus and status presentation. The consumer owns the
/// selected id, title, count/quota wording, rows and response to selection.
#[derive(IntoElement)]
pub struct SelectableList {
    id: String,
    title: SharedString,
    summary: Option<SharedString>,
    rows: Vec<SelectableRow>,
    selected: Option<String>,
    empty_message: SharedString,
    focus: SelectableListFocus,
    on_select: Option<SelectAction>,
    actions: Option<AnyElement>,
}

impl SelectableList {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<SharedString>,
        rows: Vec<SelectableRow>,
        selected: Option<String>,
        empty_message: impl Into<SharedString>,
        focus: SelectableListFocus,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            rows,
            selected,
            empty_message: empty_message.into(),
            focus,
            on_select: None,
            actions: None,
        }
    }

    pub fn summary(mut self, summary: impl Into<SharedString>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn on_select(mut self, handler: SelectAction) -> Self {
        self.on_select = Some(handler);
        self
    }

    pub fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }
}

impl RenderOnce for SelectableList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let mut list = div()
            .id(ElementId::Name(format!("{}.list", self.id).into()))
            .role(gpui::Role::ListBox)
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .overflow_y_scroll();
        for item in &self.rows {
            let selected = self.selected.as_deref() == Some(item.id.as_str());
            let status = item.status.clone().unwrap_or_default();
            let mut row = div()
                .id(ElementId::Name(
                    format!("{}.row.{}", self.id, item.id).into(),
                ))
                .role(gpui::Role::ListBoxOption)
                .aria_selected(selected)
                .aria_label(format!("{} {} {}", item.label, item.preview, status))
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
                        .child(div().text_xs().text_color(tokens.warning()).child(status)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(tokens.text())
                        .child(item.preview.clone()),
                );
            if item.selectable {
                let focus = self.focus.handle(&item.id, cx);
                let click = self.on_select.clone();
                let keyboard = self.on_select.clone();
                let click_id = item.id.clone();
                let key_id = item.id.clone();
                let focus_for_click = focus.clone();
                row = row
                    .cursor_pointer()
                    .hover(|style| style.bg(tokens.surface_raised()))
                    .track_focus(&focus)
                    .key_context(SELECTABLE_LIST_KEY_CONTEXT)
                    .on_click(move |_event, window, cx| {
                        window.focus(&focus_for_click, cx);
                        if let Some(handler) = &click {
                            handler(&click_id, window, cx);
                        }
                    })
                    .on_action(move |_: &ActivateRow, window, cx| {
                        if let Some(handler) = &keyboard {
                            handler(&key_id, window, cx);
                        }
                    });
            } else {
                row = row.opacity(0.45).cursor_default();
            }
            list = list.child(row);
        }
        let body = if self.rows.is_empty() {
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
        let mut header = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(tokens.text())
                    .child(self.title),
            );
        if let Some(summary) = self.summary {
            header = header.child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child(summary),
            );
        }
        let mut panel = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(header)
            .child(body);
        if let Some(actions) = self.actions {
            panel = panel.child(actions);
        }
        panel
    }
}
