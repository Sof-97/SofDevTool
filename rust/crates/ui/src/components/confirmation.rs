use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, App, FocusHandle, IntoElement, RenderOnce, SharedString, Window};

use crate::theme::ThemeTokens;

use super::{Button, ButtonVariant};

type ConfirmationAction = Rc<dyn Fn(&mut Window, &mut App)>;

/// Reusable inline confirmation. The consumer decides when it appears, what
/// it says and what confirming or cancelling means.
#[derive(IntoElement)]
pub struct ConfirmationBar {
    id: String,
    message: SharedString,
    confirm_label: SharedString,
    cancel_label: SharedString,
    focus: Option<[FocusHandle; 2]>,
    on_confirm: Option<ConfirmationAction>,
    on_cancel: Option<ConfirmationAction>,
}

impl ConfirmationBar {
    pub fn new(
        id: impl Into<String>,
        message: impl Into<SharedString>,
        confirm_label: impl Into<SharedString>,
        cancel_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            confirm_label: confirm_label.into(),
            cancel_label: cancel_label.into(),
            focus: None,
            on_confirm: None,
            on_cancel: None,
        }
    }

    pub fn focus_handles(mut self, confirm: FocusHandle, cancel: FocusHandle) -> Self {
        self.focus = Some([confirm, cancel]);
        self
    }

    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ConfirmationBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let mut confirm = Button::with_id(format!("{}.confirm", self.id), self.confirm_label)
            .variant(ButtonVariant::Primary);
        let mut cancel = Button::with_id(format!("{}.cancel", self.id), self.cancel_label);
        if let Some([confirm_focus, cancel_focus]) = self.focus {
            confirm = confirm.focus_handle(confirm_focus);
            cancel = cancel.focus_handle(cancel_focus);
        }
        if let Some(handler) = self.on_confirm {
            confirm = confirm.on_click(move |window, cx| handler(window, cx));
        }
        if let Some(handler) = self.on_cancel {
            cancel = cancel.on_click(move |window, cx| handler(window, cx));
        }
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_3()
            .w_full()
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(tokens.warning())
            .bg(tokens.surface_raised())
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text())
                    .child(self.message),
            )
            .child(div().flex().flex_row().gap_2().child(confirm).child(cancel))
    }
}
