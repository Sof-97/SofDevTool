use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, App, ElementId, FocusHandle, IntoElement, KeyBinding, MouseButton, RenderOnce,
    SharedString, Window,
};

use crate::theme::ThemeTokens;

gpui::actions!(sofui_hold_button, [ActivateHoldButton]);

/// A pointer/keyboard handler owned by a [`HoldButton`].
type HoldHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Key context for a [`HoldButton`]; Enter activates the keyboard confirmation.
pub const HOLD_BUTTON_KEY_CONTEXT: &str = "SofuiHoldButton";

pub(crate) fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "enter",
        ActivateHoldButton,
        Some(HOLD_BUTTON_KEY_CONTEXT),
    )]);
}

/// A destructive action that requires a deliberate pointer hold.
///
/// The host view owns the hold timer: this element only reports pointer
/// down/up/exit and keyboard activation. Keyboard activation is delivered
/// through a separate callback so the host can show a normal confirmation
/// instead of a timed hold. `progress` (0..=1) draws the hold indicator.
#[derive(IntoElement)]
pub struct HoldButton {
    label: SharedString,
    element_id: ElementId,
    progress: f32,
    disabled: bool,
    focus: Option<FocusHandle>,
    on_press: Option<HoldHandler>,
    on_release: Option<HoldHandler>,
    on_keyboard: Option<HoldHandler>,
}

impl HoldButton {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            element_id: id.into(),
            progress: 0.0,
            disabled: false,
            focus: None,
            on_press: None,
            on_release: None,
            on_keyboard: None,
        }
    }

    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus = Some(handle);
        self
    }

    /// Pointer pressed: the host starts its hold timer.
    pub fn on_press(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_press = Some(Rc::new(handler));
        self
    }

    /// Pointer released or left: the host cancels its hold timer.
    pub fn on_release(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_release = Some(Rc::new(handler));
        self
    }

    /// Keyboard activation: the host shows a confirmation.
    pub fn on_keyboard(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_keyboard = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for HoldButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let HoldButton {
            label,
            element_id,
            progress,
            disabled,
            focus,
            on_press,
            on_release,
            on_keyboard,
        } = self;

        let focused = focus
            .as_ref()
            .map(|handle| window.focused(cx).as_ref() == Some(handle))
            .unwrap_or(false);
        let border = if focused {
            tokens.accent()
        } else {
            tokens.danger()
        };

        let mut element = div()
            .id(element_id)
            .role(gpui::Role::Button)
            .aria_label(label.clone())
            .relative()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .px_3()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(tokens.surface_raised())
            .text_color(tokens.danger())
            .child(
                // The fill shows how much of the required hold has elapsed.
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(gpui::relative(progress))
                    .bg(tokens.danger())
                    .opacity(0.25),
            )
            .child(div().relative().child(label));

        if disabled {
            return element.opacity(0.45).cursor_default();
        }

        element = element.cursor_pointer();
        if let Some(press) = on_press.clone() {
            let focus_for_press = focus.clone();
            element = element.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                if let Some(handle) = &focus_for_press {
                    window.focus(handle, cx);
                }
                press(window, cx);
            });
        }
        if let Some(release) = on_release.clone() {
            let release_up = release.clone();
            element = element.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                release_up(window, cx);
            });
            // Leaving the button cancels the hold, matching "pointer exit".
            element = element.on_hover(move |hovered, window, cx| {
                if !hovered {
                    release(window, cx);
                }
            });
        }
        if let Some(handle) = focus {
            let keyboard = on_keyboard;
            element = element
                .track_focus(&handle)
                .key_context(HOLD_BUTTON_KEY_CONTEXT)
                .on_action(move |_: &ActivateHoldButton, window, cx| {
                    if let Some(handler) = &keyboard {
                        handler(window, cx);
                    }
                });
        }
        element
    }
}
