use gpui::prelude::*;
use gpui::{div, Context, IntoElement, Render, VisualTestContext, Window};
use sofui::{SegmentedControl, SegmentedControlFocus, SegmentedOption};

struct Probe {
    selected: String,
    changes: Vec<String>,
    focus: SegmentedControlFocus,
    alpha_focus: gpui::FocusHandle,
    gamma_focus: gpui::FocusHandle,
}

impl Probe {
    fn new(cx: &mut Context<Self>) -> Self {
        let focus = SegmentedControlFocus::new();
        let alpha_focus = focus.handle("public.format", "alpha", cx);
        let gamma_focus = focus.handle("public.format", "gamma", cx);
        Self {
            selected: "alpha".to_owned(),
            changes: Vec::new(),
            focus,
            alpha_focus,
            gamma_focus,
        }
    }
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.weak_entity();
        let choices = SegmentedControl::new(
            "public.format",
            "Format",
            vec![
                SegmentedOption::new("alpha", "Alpha"),
                SegmentedOption::new("beta", "Unavailable Beta").disabled(true),
                SegmentedOption::new("gamma", "Gamma"),
            ],
            Some(self.selected.clone()),
            self.focus.clone(),
        )
        .on_change(std::rc::Rc::new(move |id, _window, cx| {
            weak.update(cx, |this, cx| {
                this.selected = id.to_owned();
                this.changes.push(id.to_owned());
                cx.notify();
            })
            .ok();
        }));
        div().child(choices)
    }
}

#[gpui::test]
fn segmented_control_pointer_keyboard_wrap_and_disabled_options(cx: &mut gpui::TestAppContext) {
    cx.update(sofui::init);
    let mut captured = None;
    let window = cx.add_window(|window, cx| {
        let probe = cx.new(Probe::new);
        captured = Some(probe.clone());
        gpui_component::Root::new(probe, window, cx)
    });
    let probe = captured.unwrap();
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));

    let alpha = cx
        .debug_bounds("public.format.option.alpha")
        .unwrap()
        .center();
    cx.simulate_mouse_move(alpha, None, gpui::Modifiers::none());
    cx.simulate_mouse_down(alpha, gpui::MouseButton::Left, gpui::Modifiers::none());
    cx.simulate_mouse_up(alpha, gpui::MouseButton::Left, gpui::Modifiers::none());
    assert!(probe.read_with(&cx, |view, _| view.changes.is_empty()));

    cx.simulate_keystrokes("right");
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        "gamma"
    );
    assert_eq!(
        probe.read_with(&cx, |view, _| view.changes.clone()),
        vec!["gamma"]
    );

    cx.simulate_keystrokes("right");
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        "alpha"
    );
    cx.simulate_keystrokes("left");
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        "gamma"
    );
    cx.simulate_keystrokes("home");
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        "alpha"
    );
    cx.simulate_keystrokes("end");
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        "gamma"
    );

    let beta = cx
        .debug_bounds("public.format.option.beta")
        .unwrap()
        .center();
    cx.simulate_mouse_move(beta, None, gpui::Modifiers::none());
    cx.simulate_mouse_down(beta, gpui::MouseButton::Left, gpui::Modifiers::none());
    cx.simulate_mouse_up(beta, gpui::MouseButton::Left, gpui::Modifiers::none());
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        "gamma"
    );

    let alpha = cx
        .debug_bounds("public.format.option.alpha")
        .unwrap()
        .center();
    cx.simulate_mouse_move(alpha, None, gpui::Modifiers::none());
    cx.simulate_mouse_down(alpha, gpui::MouseButton::Left, gpui::Modifiers::none());
    cx.simulate_mouse_up(alpha, gpui::MouseButton::Left, gpui::Modifiers::none());
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        "alpha"
    );

    cx.update(|window, cx| {
        let focus = probe.read(cx).gamma_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        "gamma"
    );
    cx.update(|window, cx| {
        let focus = probe.read(cx).alpha_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("space");
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        "alpha"
    );
    assert_eq!(
        probe.read_with(&cx, |view, _| view.changes.clone()),
        vec!["gamma", "alpha", "gamma", "alpha", "gamma", "alpha", "gamma", "alpha"]
    );
}
