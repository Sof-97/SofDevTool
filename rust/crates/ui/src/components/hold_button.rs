use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, App, ElementId, FocusHandle, IntoElement, KeyBinding, MouseButton, RenderOnce,
    SharedString, Window,
};

use crate::theme::ThemeTokens;

gpui::actions!(sofui_hold_button, [ActivateHoldButton, CancelHoldButton]);

type KeyboardHandler = Rc<dyn Fn(&mut Window, &mut App)>;
type CompletionHandler = Rc<dyn Fn(&mut App)>;

/// Key context for a [`HoldButton`]. Enter or Space requests confirmation;
/// Escape cancels an active pointer hold.
pub const HOLD_BUTTON_KEY_CONTEXT: &str = "SofuiHoldButton";

pub(crate) fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", ActivateHoldButton, Some(HOLD_BUTTON_KEY_CONTEXT)),
        KeyBinding::new("space", ActivateHoldButton, Some(HOLD_BUTTON_KEY_CONTEXT)),
        KeyBinding::new("escape", CancelHoldButton, Some(HOLD_BUTTON_KEY_CONTEXT)),
    ]);
}

#[derive(Default)]
struct HoldState {
    generation: u64,
    active: bool,
    progress: f32,
}

/// Retained timing state for one destructive control. Clone it into redraws;
/// the consuming application never schedules or cancels a timer.
#[derive(Clone, Default)]
pub struct HoldController(Rc<RefCell<HoldState>>);

impl HoldController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn progress(&self) -> f32 {
        self.0.borrow().progress
    }

    pub fn is_active(&self) -> bool {
        self.0.borrow().active
    }

    fn begin(&self, duration: Duration, complete: CompletionHandler, cx: &mut App) {
        let generation = {
            let mut state = self.0.borrow_mut();
            state.generation += 1;
            state.active = true;
            state.progress = 0.0;
            state.generation
        };
        cx.refresh_windows();
        let state = self.clone();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |cx| {
            const STEPS: u32 = 20;
            for step in 1..=STEPS {
                executor.timer(duration / STEPS).await;
                let active = {
                    let mut current = state.0.borrow_mut();
                    if !current.active || current.generation != generation {
                        false
                    } else {
                        current.progress = step as f32 / STEPS as f32;
                        true
                    }
                };
                if !active {
                    return;
                }
                cx.refresh();
            }
            let completed = {
                let mut current = state.0.borrow_mut();
                if !current.active || current.generation != generation {
                    false
                } else {
                    current.active = false;
                    current.progress = 0.0;
                    current.generation += 1;
                    true
                }
            };
            if completed {
                cx.update(|cx| complete(cx));
                cx.refresh();
            }
        })
        .detach();
    }

    fn cancel(&self, cx: &mut App) {
        let mut state = self.0.borrow_mut();
        if state.active {
            state.generation += 1;
            state.active = false;
            state.progress = 0.0;
            drop(state);
            cx.refresh_windows();
        }
    }
}

/// Pointer hold with library-owned timing, progress and cancellation.
/// Keyboard or assistive activation invokes `on_keyboard` to request an
/// ordinary confirmation; it never invokes the destructive completion.
#[derive(IntoElement)]
pub struct HoldButton {
    label: SharedString,
    element_id: ElementId,
    duration: Duration,
    controller: HoldController,
    disabled: bool,
    focus: Option<FocusHandle>,
    on_complete: Option<CompletionHandler>,
    on_keyboard: Option<KeyboardHandler>,
}

impl HoldButton {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        controller: HoldController,
    ) -> Self {
        Self {
            label: label.into(),
            element_id: id.into(),
            duration: Duration::from_secs(1),
            controller,
            disabled: false,
            focus: None,
            on_complete: None,
            on_keyboard: None,
        }
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        assert!(!duration.is_zero(), "a hold needs a positive duration");
        self.duration = duration;
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

    pub fn on_complete(mut self, handler: impl Fn(&mut App) + 'static) -> Self {
        self.on_complete = Some(Rc::new(handler));
        self
    }

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
            duration,
            controller,
            disabled,
            focus,
            on_complete,
            on_keyboard,
        } = self;
        let focused = focus
            .as_ref()
            .is_some_and(|handle| window.focused(cx).as_ref() == Some(handle));
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
            .border_color(if focused {
                tokens.accent()
            } else {
                tokens.danger()
            })
            .bg(tokens.surface_raised())
            .text_color(tokens.danger())
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(gpui::relative(controller.progress()))
                    .bg(tokens.danger())
                    .opacity(0.25),
            )
            .child(div().relative().child(label));
        if disabled {
            controller.cancel(cx);
            return element.opacity(0.45).cursor_default();
        }
        element = element.cursor_pointer();
        if let Some(complete) = on_complete {
            let pressed = controller.clone();
            let released = controller.clone();
            let exited = controller.clone();
            let focus_for_press = focus.clone();
            element = element
                .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    if let Some(handle) = &focus_for_press {
                        window.focus(handle, cx);
                    }
                    pressed.begin(duration, complete.clone(), cx);
                })
                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                    released.cancel(cx);
                })
                .on_hover(move |hovered, _window, cx| {
                    if !hovered {
                        exited.cancel(cx);
                    }
                });
        }
        if let Some(handler) = on_keyboard.clone() {
            // GPUI's default accessibility Click synthesizes a short pointer
            // press. Handle it explicitly so assistive activation requests the
            // same ordinary confirmation as Enter or Space.
            element =
                element.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                    handler(window, cx);
                });
        }
        if let Some(handle) = focus {
            let escape = controller;
            element = element
                .track_focus(&handle)
                .key_context(HOLD_BUTTON_KEY_CONTEXT)
                .on_action(move |_: &ActivateHoldButton, window, cx| {
                    if let Some(handler) = &on_keyboard {
                        handler(window, cx);
                    }
                })
                .on_action(move |_: &CancelHoldButton, _window, cx| escape.cancel(cx));
        }
        element
    }
}
