use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, App, ElementId, FocusHandle, IntoElement, KeyBinding, RenderOnce, SharedString, Window,
};

use crate::theme::ThemeTokens;

/// Key context for focusable buttons; Enter and Space activate the focused one.
pub const BUTTON_KEY_CONTEXT: &str = "SofuiButton";

gpui::actions!(sofui_button, [ActivateButton]);

/// A click or keyboard activation handler owned by a [`Button`].
pub type ClickHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
}

#[derive(IntoElement)]
pub struct Button {
    label: SharedString,
    aria_label: Option<SharedString>,
    element_id: ElementId,
    variant: ButtonVariant,
    disabled: bool,
    focus: Option<FocusHandle>,
    on_click: Option<ClickHandler>,
}

impl Button {
    /// Constructs a button with a caller-owned stable identity. Keep `id`
    /// unchanged across redraws, even when the visible label changes. Distinct
    /// controls sharing one label must use distinct ids.
    pub fn with_id(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            aria_label: None,
            element_id: id.into(),
            variant: ButtonVariant::Secondary,
            disabled: false,
            focus: None,
            on_click: None,
        }
    }

    /// Supplies a descriptive accessibility label when the visible label is
    /// symbolic, such as the plus or minus button of a numeric control.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Primary variant with stable identity independent of its label.
    pub fn primary_with_id(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::with_id(id, label).variant(ButtonVariant::Primary)
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Makes the button focusable and reachable with Tab/Shift-Tab. The handle
    /// must be retained by the view so focus survives redraws. Enter and Space
    /// invoke the same action as a pointer click. A disabled button invokes none.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus = Some(handle);
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

pub(crate) fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", ActivateButton, Some(BUTTON_KEY_CONTEXT)),
        KeyBinding::new("space", ActivateButton, Some(BUTTON_KEY_CONTEXT)),
    ]);
}

/// Adapts a view-context handler into the event-less [`Button`] handler, so
/// callers can mutate their view without naming a GPUI event type.
pub fn view_click<T: 'static>(
    cx: &mut gpui::Context<T>,
    handler: impl Fn(&mut T, &mut Window, &mut gpui::Context<T>) + 'static,
) -> impl Fn(&mut Window, &mut App) + 'static {
    let weak = cx.weak_entity();
    move |window, cx| {
        weak.update(cx, |this, cx| handler(this, window, cx)).ok();
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Button {
            label,
            aria_label,
            element_id,
            variant,
            disabled,
            focus,
            on_click,
        } = self;
        let tokens = ThemeTokens::active();
        let (background, foreground, base_border) = match variant {
            ButtonVariant::Primary => (tokens.accent(), tokens.accent_text(), tokens.accent()),
            ButtonVariant::Secondary => (tokens.surface_raised(), tokens.text(), tokens.border()),
        };

        let focused = focus
            .as_ref()
            .map(|handle| window.focused(cx).as_ref() == Some(handle))
            .unwrap_or(false);
        let border = if focused {
            // A visible focus ring distinct from each variant's resting border,
            // including Primary whose resting border is the accent colour.
            match variant {
                ButtonVariant::Primary => tokens.text(),
                ButtonVariant::Secondary => tokens.accent(),
            }
        } else {
            base_border
        };

        let mut element = div()
            .id(element_id)
            .role(gpui::Role::Button)
            .aria_label(aria_label.unwrap_or_else(|| label.clone()))
            .flex()
            .items_center()
            .justify_center()
            .px_3()
            .py_1()
            .rounded_md()
            .border_1()
            .bg(background)
            .text_color(foreground)
            .border_color(border)
            .child(label);

        if disabled {
            element = element.opacity(0.45).cursor_default();
        } else {
            element = element.cursor_pointer().hover(|style| style.opacity(0.85));
            match focus {
                Some(handle) => {
                    let for_keyboard = on_click.clone();
                    let for_mouse = on_click;
                    let focus_for_mouse = handle.clone();
                    element = element
                        .track_focus(&handle)
                        .key_context(BUTTON_KEY_CONTEXT)
                        .on_action(move |_: &ActivateButton, window, cx| {
                            if let Some(handler) = &for_keyboard {
                                handler(window, cx);
                            }
                        })
                        .on_click(move |_event, window, cx| {
                            window.focus(&focus_for_mouse, cx);
                            if let Some(handler) = &for_mouse {
                                handler(window, cx);
                            }
                        });
                }
                None => {
                    if let Some(handler) = on_click {
                        element = element.on_click(move |_event, window, cx| handler(window, cx));
                    }
                }
            }
        }

        element
    }
}
