use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, point, px, Context, FocusHandle, IntoElement, Modifiers, MouseButton, Render,
    VisualTestContext, Window,
};
use sofui::{
    view_click, ConfirmationBar, HoldButton, HoldController, SelectableList, SelectableListFocus,
    SelectableRow,
};

struct Probe {
    hold: HoldController,
    hold_focus: FocusHandle,
    confirm_focus: FocusHandle,
    cancel_focus: FocusHandle,
    list_focus: SelectableListFocus,
    selected: Option<String>,
    completions: usize,
    requests: usize,
    confirmed: usize,
}

impl Probe {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            hold: HoldController::new(),
            hold_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            confirm_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            cancel_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            list_focus: SelectableListFocus::new(),
            selected: None,
            completions: 0,
            requests: 0,
            confirmed: 0,
        }
    }
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.weak_entity();
        let list = SelectableList::new(
            "public.list",
            "Records",
            vec![
                SelectableRow {
                    id: "a".into(),
                    label: "First".into(),
                    preview: "Available".into(),
                    status: None,
                    selectable: true,
                },
                SelectableRow {
                    id: "b".into(),
                    label: "Second".into(),
                    preview: "Unreadable".into(),
                    status: Some("Unavailable".into()),
                    selectable: true,
                },
                SelectableRow {
                    id: "c".into(),
                    label: "Third".into(),
                    preview: "Failed".into(),
                    status: Some("Failed".into()),
                    selectable: false,
                },
            ],
            self.selected.clone(),
            "Nothing here",
            self.list_focus.clone(),
        )
        .summary("3/7")
        .on_select(std::rc::Rc::new(move |id, _window, cx| {
            weak.update(cx, |this, cx| {
                this.selected = Some(id.to_owned());
                cx.notify();
            })
            .ok();
        }));
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div().debug_selector(|| "hold-hitbox".into()).child(
                    HoldButton::new("public.hold", "Remove", self.hold.clone())
                        .duration(Duration::from_secs(1))
                        .focus_handle(self.hold_focus.clone())
                        .on_complete({
                            let weak = cx.weak_entity();
                            move |cx| {
                                weak.update(cx, |this, cx| {
                                    this.completions += 1;
                                    cx.notify();
                                })
                                .ok();
                            }
                        })
                        .on_keyboard(view_click(cx, |this, _window, cx| {
                            this.requests += 1;
                            cx.notify();
                        })),
                ),
            )
            .when(self.requests > self.confirmed, |this| {
                this.child(
                    ConfirmationBar::new("public.confirm", "Remove this item?", "Remove", "Cancel")
                        .focus_handles(self.confirm_focus.clone(), self.cancel_focus.clone())
                        .on_confirm(view_click(cx, |this, _window, cx| {
                            this.confirmed += 1;
                            cx.notify();
                        }))
                        .on_cancel(view_click(cx, |this, _window, cx| {
                            this.requests = this.confirmed;
                            cx.notify();
                        })),
                )
            })
            .child(div().h_48().child(list))
    }
}

fn advance_hold(cx: &mut VisualTestContext, ticks: usize) {
    for _ in 0..ticks {
        cx.executor().advance_clock(Duration::from_millis(50));
        cx.run_until_parked();
    }
}

#[gpui::test]
fn pointer_hold_cancels_on_release_exit_and_escape_then_completes_once(
    cx: &mut gpui::TestAppContext,
) {
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
    let hit = cx.debug_bounds("hold-hitbox").unwrap().center();
    cx.simulate_mouse_move(hit, None, Modifiers::none());
    cx.simulate_mouse_down(hit, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    advance_hold(&mut cx, 5);
    assert!(probe.read_with(&cx, |view, _| view.hold.progress()) > 0.0);
    cx.simulate_mouse_up(hit, MouseButton::Left, Modifiers::none());
    advance_hold(&mut cx, 20);
    assert_eq!(probe.read_with(&cx, |view, _| view.completions), 0);
    assert!(!probe.read_with(&cx, |view, _| view.hold.is_active()));

    cx.simulate_mouse_down(hit, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    advance_hold(&mut cx, 5);
    cx.simulate_mouse_move(
        point(px(900.), px(700.)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    advance_hold(&mut cx, 20);
    assert_eq!(probe.read_with(&cx, |view, _| view.completions), 0);
    assert!(!probe.read_with(&cx, |view, _| view.hold.is_active()));

    cx.simulate_mouse_move(hit, None, Modifiers::none());
    cx.simulate_mouse_down(hit, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    advance_hold(&mut cx, 5);
    cx.simulate_keystrokes("escape");
    advance_hold(&mut cx, 20);
    assert_eq!(probe.read_with(&cx, |view, _| view.completions), 0);

    cx.simulate_mouse_move(hit, None, Modifiers::none());
    cx.simulate_mouse_down(hit, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    advance_hold(&mut cx, 20);
    assert_eq!(probe.read_with(&cx, |view, _| view.completions), 1);
    cx.simulate_mouse_up(hit, MouseButton::Left, Modifiers::none());
    advance_hold(&mut cx, 20);
    assert_eq!(probe.read_with(&cx, |view, _| view.completions), 1);
}

#[gpui::test]
fn keyboard_requests_confirmation_and_list_keeps_focus_after_selection(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(sofui::init);
    let mut captured = None;
    let window = cx.add_window(|window, cx| {
        let probe = cx.new(Probe::new);
        captured = Some(probe.clone());
        gpui_component::Root::new(probe, window, cx)
    });
    let probe = captured.unwrap();
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let focus = probe.read(cx).hold_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(probe.read_with(&cx, |view, _| view.requests), 1);
    assert_eq!(probe.read_with(&cx, |view, _| view.completions), 0);
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let focus = probe.read(cx).cancel_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(probe.read_with(&cx, |view, _| view.requests), 0);
    cx.update(|window, cx| {
        let focus = probe.read(cx).hold_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("space");
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let focus = probe.read(cx).confirm_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(probe.read_with(&cx, |view, _| view.confirmed), 1);
    assert_eq!(probe.read_with(&cx, |view, _| view.completions), 0);

    cx.update(|window, cx| {
        let state = probe.read(cx).list_focus.clone();
        let focus = state.handle("b", cx);
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(
        probe.read_with(&cx, |view, _| view.selected.clone()),
        Some("b".to_owned())
    );
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let state = probe.read(cx).list_focus.clone();
        let focus = state.handle("b", cx);
        assert!(focus.is_focused(window));
    });
}
