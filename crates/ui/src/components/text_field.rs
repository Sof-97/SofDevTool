use gpui::prelude::*;
use gpui::{App, Context, Entity, Focusable, IntoElement, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};

/// A single-line text field owned by this library.
///
/// `gpui-component`'s editing engine is an implementation detail: callers only
/// see this type's assignment/edit, text, change subscription and render surface.
pub struct TextField {
    state: Entity<InputState>,
}

impl TextField {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        Self {
            state: cx.new(|cx| InputState::new(window, cx)),
        }
    }

    /// Silently assigns text for initialization or restoration. This emits no
    /// change notification and clears the editor's undo history. A caller that
    /// needs to evaluate the new value must request that work explicitly.
    pub fn assign_text(&self, text: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.set_value(text, window, cx));
    }

    /// Replaces the whole text as a user-style edit: recorded in the undo
    /// history and emitting a change event.
    pub fn edit_text(&self, text: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
        self.state
            .update(cx, |state, cx| state.replace_all(text, window, cx));
    }

    /// Compatibility alias for silent assignment.
    pub fn set_text(&self, text: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
        self.assign_text(text, window, cx);
    }

    /// Compatibility alias for a user-style edit.
    pub fn replace_all(&self, text: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
        self.edit_text(text, window, cx);
    }

    pub fn text(&self, cx: &App) -> String {
        self.state.read(cx).value().to_string()
    }

    /// Gives a host view the real editor focus after its window opens.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.state.read(cx).focus_handle(cx).focus(window, cx);
    }

    /// Subscribe inside a view's context. The handler receives the view, window
    /// and context so it can recompute and notify without naming the engine type.
    pub fn on_change_in<T: 'static>(
        &self,
        window: &Window,
        cx: &mut Context<T>,
        mut handler: impl FnMut(&mut T, &mut Window, &mut Context<T>) + 'static,
    ) -> Subscription {
        cx.subscribe_in(
            &self.state,
            window,
            move |view, _entity, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    handler(view, window, cx);
                }
            },
        )
    }

    pub fn render(&self, accessibility_id: &'static str) -> impl IntoElement {
        Input::new(&self.state)
            .accessibility_id(accessibility_id)
            .w_full()
    }
}
