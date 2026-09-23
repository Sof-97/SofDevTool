use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, App, ElementId, FocusHandle, IntoElement, KeyBinding, RenderOnce, SharedString, Window,
};

use crate::theme::ThemeTokens;

gpui::actions!(
    sofui_segmented_control,
    [
        PreviousOption,
        NextOption,
        FirstOption,
        LastOption,
        ActivateOption
    ]
);

/// Key context for a [`SegmentedControl`]. Arrow keys move and select; Enter
/// and Space activate the focused option.
pub const SEGMENTED_CONTROL_KEY_CONTEXT: &str = "SofuiSegmentedControl";

pub(crate) fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", PreviousOption, Some(SEGMENTED_CONTROL_KEY_CONTEXT)),
        KeyBinding::new("right", NextOption, Some(SEGMENTED_CONTROL_KEY_CONTEXT)),
        KeyBinding::new("home", FirstOption, Some(SEGMENTED_CONTROL_KEY_CONTEXT)),
        KeyBinding::new("end", LastOption, Some(SEGMENTED_CONTROL_KEY_CONTEXT)),
        KeyBinding::new("enter", ActivateOption, Some(SEGMENTED_CONTROL_KEY_CONTEXT)),
        KeyBinding::new("space", ActivateOption, Some(SEGMENTED_CONTROL_KEY_CONTEXT)),
    ]);
}

/// One caller-owned choice. `id` must stay stable when the visible label changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentedOption {
    pub id: String,
    pub label: String,
    pub disabled: bool,
}

impl SegmentedOption {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Retains a distinct focus handle for each stable group and option identity.
#[derive(Clone, Default)]
pub struct SegmentedControlFocus(Rc<RefCell<BTreeMap<String, BTreeMap<String, FocusHandle>>>>);

impl SegmentedControlFocus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle(&self, group_id: &str, option_id: &str, cx: &mut App) -> FocusHandle {
        self.0
            .borrow_mut()
            .entry(group_id.to_owned())
            .or_default()
            .entry(option_id.to_owned())
            .or_insert_with(|| cx.focus_handle().tab_stop(true).tab_index(0))
            .clone()
    }
}

pub type SegmentChange = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// A generic, accessible choice group. The consumer owns option meaning and
/// selected state; arrow keys move through enabled options and report the
/// resulting stable option ID through `on_change`.
#[derive(IntoElement)]
pub struct SegmentedControl {
    id: String,
    label: SharedString,
    options: Vec<SegmentedOption>,
    selected: Option<String>,
    focus: SegmentedControlFocus,
    on_change: Option<SegmentChange>,
}

impl SegmentedControl {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<SharedString>,
        options: Vec<SegmentedOption>,
        selected: Option<String>,
        focus: SegmentedControlFocus,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            options,
            selected,
            focus,
            on_change: None,
        }
    }

    pub fn on_change(mut self, handler: SegmentChange) -> Self {
        self.on_change = Some(handler);
        self
    }
}

fn navigation_target(
    options: &[SegmentedOption],
    current: usize,
    action: Navigation,
) -> Option<usize> {
    if options.is_empty() {
        return None;
    }
    match action {
        Navigation::First => options.iter().position(|option| !option.disabled),
        Navigation::Last => options.iter().rposition(|option| !option.disabled),
        Navigation::Previous | Navigation::Next => {
            let step = if action == Navigation::Previous {
                -1_isize
            } else {
                1
            };
            (1..=options.len())
                .map(|offset| {
                    (current as isize + step * offset as isize).rem_euclid(options.len() as isize)
                        as usize
                })
                .find(|index| !options[*index].disabled)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Navigation {
    Previous,
    Next,
    First,
    Last,
}

impl RenderOnce for SegmentedControl {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let options = Rc::new(self.options);
        let option_count = options.len();
        let mut group = div()
            .id(ElementId::Name(format!("{}.group", self.id).into()))
            .role(gpui::Role::RadioGroup)
            .aria_label(self.label)
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_2();

        for (index, option) in options.iter().enumerate() {
            let selected = self.selected.as_deref() == Some(option.id.as_str());
            let focus = self.focus.handle(&self.id, &option.id, cx);
            let focused = window.focused(cx).as_ref() == Some(&focus);
            let disabled = option.disabled;
            let mut border = if focused {
                tokens.text()
            } else if selected {
                tokens.accent()
            } else {
                tokens.border()
            };
            if disabled {
                border = tokens.border();
            }
            let background = if selected {
                tokens.accent()
            } else if focused {
                tokens.surface_raised()
            } else {
                tokens.surface()
            };
            let foreground = if selected {
                tokens.accent_text()
            } else {
                tokens.text()
            };
            let id = option.id.clone();
            let debug_selector = format!("{}.option.{}", self.id, option.id);
            let selected_id = self.selected.clone();
            let options_for_click = options.clone();
            let callback = self.on_change.clone();
            let focus_for_click = focus.clone();
            let mut item = div()
                .id(ElementId::Name(
                    format!("{}.option.{}", self.id, option.id).into(),
                ))
                .debug_selector(move || debug_selector.clone())
                .role(gpui::Role::RadioButton)
                .aria_label(option.label.clone())
                .aria_selected(selected)
                .aria_position_in_set(index + 1)
                .aria_size_of_set(option_count)
                .when(disabled, |item| item.aria_description("Unavailable"))
                .flex()
                .items_center()
                .justify_center()
                .px_3()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(border)
                .bg(background)
                .text_color(foreground)
                .child(option.label.clone());

            if disabled {
                item = item.opacity(0.45).cursor_default();
            } else {
                let keyboard_options = options_for_click.clone();
                let keyboard_selected = selected_id.clone();
                let keyboard_change = callback.clone();
                let keyboard_focus = self.focus.clone();
                let group_id = self.id.clone();
                item = item
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.85))
                    .track_focus(&focus)
                    .key_context(SEGMENTED_CONTROL_KEY_CONTEXT)
                    .on_click(move |_event, window, cx| {
                        window.focus(&focus_for_click, cx);
                        if selected_id.as_deref() != Some(id.as_str()) {
                            if let Some(handler) = &callback {
                                handler(&id, window, cx);
                            }
                        }
                    })
                    .on_action(move |_: &ActivateOption, window, cx| {
                        activate_option(
                            &keyboard_options,
                            index,
                            &group_id,
                            &keyboard_focus,
                            keyboard_selected.as_deref(),
                            keyboard_change.as_ref(),
                            window,
                            cx,
                        );
                    });
                let options = options_for_click.clone();
                let focus = self.focus.clone();
                let group_id = self.id.clone();
                let selected = self.selected.clone();
                let change = self.on_change.clone();
                item = item.on_action(move |_: &PreviousOption, window, cx| {
                    navigate_option(
                        &options,
                        index,
                        Navigation::Previous,
                        &group_id,
                        &focus,
                        selected.as_deref(),
                        change.as_ref(),
                        window,
                        cx,
                    );
                });
                let options = options_for_click.clone();
                let focus = self.focus.clone();
                let group_id = self.id.clone();
                let selected = self.selected.clone();
                let change = self.on_change.clone();
                item = item.on_action(move |_: &NextOption, window, cx| {
                    navigate_option(
                        &options,
                        index,
                        Navigation::Next,
                        &group_id,
                        &focus,
                        selected.as_deref(),
                        change.as_ref(),
                        window,
                        cx,
                    );
                });
                let options = options_for_click.clone();
                let focus = self.focus.clone();
                let group_id = self.id.clone();
                let selected = self.selected.clone();
                let change = self.on_change.clone();
                item = item.on_action(move |_: &FirstOption, window, cx| {
                    navigate_option(
                        &options,
                        index,
                        Navigation::First,
                        &group_id,
                        &focus,
                        selected.as_deref(),
                        change.as_ref(),
                        window,
                        cx,
                    );
                });
                let options = options_for_click.clone();
                let focus = self.focus.clone();
                let group_id = self.id.clone();
                let selected = self.selected.clone();
                let change = self.on_change.clone();
                item = item.on_action(move |_: &LastOption, window, cx| {
                    navigate_option(
                        &options,
                        index,
                        Navigation::Last,
                        &group_id,
                        &focus,
                        selected.as_deref(),
                        change.as_ref(),
                        window,
                        cx,
                    );
                });
            }
            group = group.child(item);
        }
        group
    }
}

#[allow(clippy::too_many_arguments)]
fn navigate_option(
    options: &[SegmentedOption],
    current: usize,
    action: Navigation,
    group_id: &str,
    focus: &SegmentedControlFocus,
    selected: Option<&str>,
    on_change: Option<&SegmentChange>,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(target) = navigation_target(options, current, action) {
        activate_option(
            options, target, group_id, focus, selected, on_change, window, cx,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn activate_option(
    options: &[SegmentedOption],
    target: usize,
    group_id: &str,
    focus: &SegmentedControlFocus,
    selected: Option<&str>,
    on_change: Option<&SegmentChange>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(option) = options.get(target).filter(|option| !option.disabled) else {
        return;
    };
    let target_id = option.id.clone();
    let target_focus = focus.handle(group_id, &target_id, cx);
    window.focus(&target_focus, cx);
    if selected != Some(target_id.as_str()) {
        if let Some(handler) = on_change {
            handler(&target_id, window, cx);
        }
    }
}
