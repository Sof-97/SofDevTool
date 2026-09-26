use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, App, FocusHandle, IntoElement, RenderOnce, Window};

use crate::theme::ThemeTokens;

use super::Button;

/// A signed step request from a numeric control.
pub type StepHandler = Rc<dyn Fn(i32, &mut Window, &mut App)>;

/// A bounded numeric control. The caller owns the value and interprets step
/// requests against its latest state; this component owns presentation, focus,
/// disabled boundaries, and pointer/keyboard activation.
#[derive(IntoElement)]
pub struct NumericStepper {
    id: String,
    label: String,
    value: Option<i32>,
    min: i32,
    max: i32,
    step: i32,
    focus: Option<[FocusHandle; 2]>,
    on_step: Option<StepHandler>,
}

impl NumericStepper {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        value: Option<i32>,
        min: i32,
        max: i32,
        step: i32,
    ) -> Self {
        assert!(min <= max, "numeric stepper needs an ordered range");
        assert!(step > 0, "numeric stepper needs a positive step");
        Self {
            id: id.into(),
            label: label.into(),
            value,
            min,
            max,
            step,
            focus: None,
            on_step: None,
        }
    }

    /// Retain both handles in the owning view across redraws.
    pub fn focus_handles(mut self, decrease: FocusHandle, increase: FocusHandle) -> Self {
        self.focus = Some([decrease, increase]);
        self
    }

    /// Receives a signed step, not a captured next value. Consumers must apply
    /// it to their latest requested state, which may change before redraw.
    pub fn on_step(mut self, handler: impl Fn(i32, &mut Window, &mut App) + 'static) -> Self {
        self.on_step = Some(Rc::new(handler));
        self
    }

    /// Saturating, bounded stepping for use by consumers when applying a step
    /// to their current state. No domain-specific quantization occurs here.
    pub fn stepped(value: i32, delta: i32, min: i32, max: i32) -> i32 {
        value.saturating_add(delta).clamp(min, max)
    }
}

impl RenderOnce for NumericStepper {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let NumericStepper {
            id,
            label,
            value,
            min,
            max,
            step,
            focus,
            on_step,
        } = self;
        let minus_id = format!("{id}.decrease");
        let plus_id = format!("{id}.increase");
        let minus_action = on_step.clone();
        let plus_action = on_step;
        let minus_focus = focus.as_ref().map(|handles| handles[0].clone());
        let plus_focus = focus.as_ref().map(|handles| handles[1].clone());
        let minus_disabled = value.is_none_or(|value| value <= min);
        let plus_disabled = value.is_none_or(|value| value >= max);
        let label_minus = format!("Decrease {label}");
        let label_plus = format!("Increase {label}");
        let tokens = ThemeTokens::active();
        let mut minus = Button::with_id(minus_id, "−")
            .aria_label(label_minus)
            .disabled(minus_disabled);
        if let Some(focus) = minus_focus {
            minus = minus.focus_handle(focus);
        }
        if let Some(action) = minus_action {
            minus = minus.on_click(move |window, cx| action(-step, window, cx));
        }
        let mut plus = Button::with_id(plus_id, "+")
            .aria_label(label_plus)
            .disabled(plus_disabled);
        if let Some(focus) = plus_focus {
            plus = plus.focus_handle(focus);
        }
        if let Some(action) = plus_action {
            plus = plus.on_click(move |window, cx| action(step, window, cx));
        }
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex_shrink_0()
                    .whitespace_nowrap()
                    .text_xs()
                    .child(label),
            )
            .child(minus)
            .child(
                div()
                    .w_10()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child(value.map_or_else(|| "—".to_owned(), |value| value.to_string())),
            )
            .child(plus)
    }
}
