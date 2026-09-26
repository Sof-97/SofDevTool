//! A separate GPUI application using only sofui's declared public interface.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, App, Context, FocusHandle, IntoElement, Render, Subscription, Window, WindowOptions,
};
use sofui::{
    apply_theme, init, mount, panel, view_click, Button, SegmentedControl, SegmentedControlFocus,
    SegmentedOption, TextEditor, ThemeVariant,
};

struct Consumer {
    editor: TextEditor,
    choice_focus: SegmentedControlFocus,
    count_focus: FocusHandle,
    layout: String,
    edits: usize,
    presses: usize,
    _edit_subscription: Subscription,
}

impl Consumer {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = TextEditor::new(window, cx);
        editor.assign_text("Independent café 👩🏽‍💻", window, cx);
        let edit_subscription = editor.on_change_in(window, cx, |this, _window, cx| {
            this.edits += 1;
            cx.notify();
        });
        Self {
            editor,
            choice_focus: SegmentedControlFocus::new(),
            count_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            layout: "compact".into(),
            edits: 0,
            presses: 0,
            _edit_subscription: edit_subscription,
        }
    }
}

impl Render for Consumer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.weak_entity();
        let layout = SegmentedControl::new(
            "consumer.layout",
            "Layout",
            vec![
                SegmentedOption::new("compact", "Compact"),
                SegmentedOption::new("comfortable", "Comfortable"),
            ],
            Some(self.layout.clone()),
            self.choice_focus.clone(),
        )
        .on_change(Rc::new(move |id, _window, cx| {
            weak.update(cx, |this, cx| {
                this.layout = id.to_owned();
                cx.notify();
            })
            .ok();
        }));

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_3()
            .child(layout)
            .child(panel(
                "Independent editor",
                format!("{} edits", self.edits),
                self.editor.render(false, "consumer.editor"),
            ))
            .child(
                Button::with_id("consumer.count", format!("Pressed {} times", self.presses))
                    .focus_handle(self.count_focus.clone())
                    .on_click(view_click(cx, |this, _window, cx| {
                        this.presses += 1;
                        cx.notify();
                    })),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        init(cx);
        apply_theme(ThemeVariant::Graphite, cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| Consumer::new(window, cx));
            mount(view, window, cx)
        })
        .expect("open independent consumer window");
    });
}
