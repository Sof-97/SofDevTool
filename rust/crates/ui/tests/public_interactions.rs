use gpui::prelude::*;
use gpui::{
    div, Context, FocusHandle, IntoElement, Render, Subscription, VisualTestContext, Window,
};
use sofui::{view_click, Button, TextEditor, TextField};

struct Probe {
    field: TextField,
    editor: TextEditor,
    first_focus: FocusHandle,
    second_focus: FocusHandle,
    disabled_focus: FocusHandle,
    label: &'static str,
    first_count: usize,
    second_count: usize,
    change_count: usize,
    _subscriptions: Vec<Subscription>,
}

impl Probe {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = TextField::new(window, cx);
        let editor = TextEditor::new(window, cx);
        let subscriptions = vec![
            field.on_change_in(window, cx, |this, _, cx| {
                this.change_count += 1;
                cx.notify();
            }),
            editor.on_change_in(window, cx, |this, _, cx| {
                this.change_count += 1;
                cx.notify();
            }),
        ];
        Self {
            field,
            editor,
            first_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            second_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            disabled_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            label: "Copy",
            first_count: 0,
            second_count: 0,
            change_count: 0,
            _subscriptions: subscriptions,
        }
    }
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(
                Button::with_id("first", self.label)
                    .focus_handle(self.first_focus.clone())
                    .on_click(view_click(cx, |this, _, cx| {
                        this.first_count += 1;
                        cx.notify();
                    })),
            )
            .child(
                Button::with_id("second", self.label)
                    .focus_handle(self.second_focus.clone())
                    .on_click(view_click(cx, |this, _, cx| {
                        this.second_count += 1;
                        cx.notify();
                    })),
            )
            .child(
                Button::with_id("disabled", self.label)
                    .focus_handle(self.disabled_focus.clone())
                    .disabled(true)
                    .on_click(view_click(cx, |this, _, cx| {
                        this.first_count += 100;
                        cx.notify();
                    })),
            )
            .child(self.field.render("test.field"))
            .child(div().h_32().child(self.editor.render(false, "test.editor")))
    }
}

#[gpui::test]
fn repeated_labels_keep_distinct_keyboard_actions_across_redraw(cx: &mut gpui::TestAppContext) {
    cx.update(sofui::init);
    let mut captured = None;
    let window = cx.add_window(|window, cx| {
        let probe = cx.new(|cx| Probe::new(window, cx));
        captured = Some(probe.clone());
        gpui_component::Root::new(probe, window, cx)
    });
    let probe = captured.unwrap();
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));

    cx.update(|window, cx| {
        let focus = probe.read(cx).first_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(probe.read_with(&cx, |probe, _| probe.first_count), 1);

    probe.update(&mut cx, |probe, cx| {
        probe.label = "Copy changed";
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(cx.update(|window, cx| probe.read(cx).first_focus.is_focused(window)));
    cx.simulate_keystrokes("space");
    assert_eq!(probe.read_with(&cx, |probe, _| probe.first_count), 2);

    cx.update(|window, cx| {
        let focus = probe.read(cx).second_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(probe.read_with(&cx, |probe, _| probe.second_count), 1);

    cx.update(|window, cx| {
        let focus = probe.read(cx).disabled_focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(probe.read_with(&cx, |probe, _| probe.first_count), 2);
}

#[gpui::test]
fn assignment_is_silent_and_editing_emits_changes_with_unicode(cx: &mut gpui::TestAppContext) {
    cx.update(sofui::init);
    let mut captured = None;
    let window = cx.add_window(|window, cx| {
        let probe = cx.new(|cx| Probe::new(window, cx));
        captured = Some(probe.clone());
        gpui_component::Root::new(probe, window, cx)
    });
    let probe = captured.unwrap();
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| {
        probe.update(cx, |probe, cx| {
            probe.field.assign_text("café", window, cx);
            probe.editor.assign_text("👨‍👩‍👧‍👦", window, cx);
        });
    });
    assert_eq!(probe.read_with(&cx, |probe, _| probe.change_count), 0);
    assert_eq!(
        probe.read_with(&cx, |probe, cx| probe.editor.text(cx)),
        "👨‍👩‍👧‍👦"
    );

    cx.update(|window, cx| {
        probe.update(cx, |probe, cx| {
            probe.field.edit_text("e\u{301}", window, cx);
            probe.editor.edit_text("🏳️‍🌈", window, cx);
        });
    });
    assert_eq!(probe.read_with(&cx, |probe, _| probe.change_count), 2);
    assert_eq!(
        probe.read_with(&cx, |probe, cx| probe.field.text(cx)),
        "e\u{301}"
    );
    assert_eq!(
        probe.read_with(&cx, |probe, cx| probe.editor.text(cx)),
        "🏳️‍🌈"
    );

    cx.update(|window, cx| {
        probe.update(cx, |probe, cx| probe.editor.focus(window, cx));
        window.draw(cx).clear(cx);
    });
    cx.simulate_keystrokes("cmd-z");
    assert_eq!(
        probe.read_with(&cx, |probe, cx| probe.editor.text(cx)),
        "👨‍👩‍👧‍👦"
    );

    cx.update(|window, cx| {
        probe.update(cx, |probe, cx| {
            probe.editor.assign_text("restored", window, cx)
        });
    });
    cx.simulate_keystrokes("cmd-z");
    assert_eq!(
        probe.read_with(&cx, |probe, cx| probe.editor.text(cx)),
        "restored"
    );
}
