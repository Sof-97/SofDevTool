//! Executable gallery for the owner-maintained component library.
//!
//! It demonstrates the actual exported components in their normal, focused,
//! disabled, invalid and empty states, plus the History list, destructive hold
//! action, confirmation and theme switch used by the application. It builds
//! without the SofDevTool application crate.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, App, ClipboardItem, Context, FocusHandle, IntoElement, Render, Subscription, Window,
    WindowOptions,
};
use sofui::{
    active_theme, copy_feedback, diagnostic_banner, empty_state, init, mount, panel,
    set_active_theme, set_dark_theme, view_click, Button, ButtonVariant, DiagnosticSeverity,
    HistoryItem, HistoryPanel, HoldButton, LabeledField, TextEditor, TextField, ThemeTokens,
    ThemeVariant,
};

struct Gallery {
    field: TextField,
    input: TextEditor,
    result: TextEditor,
    primary_focus: FocusHandle,
    secondary_focus: FocusHandle,
    variant_focus: FocusHandle,
    theme_focus: FocusHandle,
    hold_focus: FocusHandle,
    confirm_focus: FocusHandle,
    cancel_focus: FocusHandle,
    copied: bool,
    history_selected: Option<String>,
    history_items: Vec<HistoryItem>,
    holding: bool,
    hold_generation: u64,
    hold_progress: f32,
    confirm_visible: bool,
    confirmed: bool,
    _subscriptions: Vec<Subscription>,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = TextField::new(window, cx);
        let input = TextEditor::new(window, cx);
        let result = TextEditor::new(window, cx);
        field.assign_text("café", window, cx);
        input.assign_text("👨‍👩‍👧‍👦 and e\u{301} stay editable", window, cx);
        result.assign_text("Selectable result", window, cx);
        let subscriptions = vec![
            field.on_change_in(window, cx, |this, _window, cx| {
                this.copied = false;
                cx.notify();
            }),
            input.on_change_in(window, cx, |_this, _window, cx| {
                cx.notify();
            }),
        ];
        let history_items = vec![
            HistoryItem {
                id: "a".into(),
                label: "2026-09-22T10:00:00Z".into(),
                preview: "SHA-256 · ba7816bf8f01cfea…".into(),
                available: true,
            },
            HistoryItem {
                id: "b".into(),
                label: "2026-09-22T09:59:00Z".into(),
                preview: "Unavailable snapshot".into(),
                available: false,
            },
        ];
        Self {
            field,
            input,
            result,
            primary_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            secondary_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            variant_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            theme_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            hold_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            confirm_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            cancel_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            copied: false,
            history_selected: Some("a".into()),
            history_items,
            holding: false,
            hold_generation: 0,
            hold_progress: 0.0,
            confirm_visible: false,
            confirmed: false,
            _subscriptions: subscriptions,
        }
    }

    fn begin_hold(&mut self, cx: &mut Context<Self>) {
        self.hold_generation += 1;
        let generation = self.hold_generation;
        self.holding = true;
        self.hold_progress = 0.0;
        cx.notify();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            const STEPS: u32 = 20;
            for step in 1..=STEPS {
                executor.timer(Duration::from_secs(1) / STEPS).await;
                let alive = this
                    .update(cx, |this, cx| {
                        if this.hold_generation != generation {
                            return false;
                        }
                        this.hold_progress = step as f32 / STEPS as f32;
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !alive {
                    return;
                }
            }
            this.update(cx, |this, cx| {
                if this.hold_generation == generation {
                    this.holding = false;
                    this.hold_progress = 0.0;
                    this.confirmed = true;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn cancel_hold(&mut self, cx: &mut Context<Self>) {
        self.hold_generation += 1;
        self.holding = false;
        self.hold_progress = 0.0;
        cx.notify();
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let selected = self.history_selected.clone();
        let weak = cx.weak_entity();
        let history_panel = HistoryPanel::new(
            self.history_items.clone(),
            selected,
            "No retained operations yet.",
        )
        .on_select(std::rc::Rc::new(move |id, _window, cx| {
            weak.update(cx, |this, cx| {
                this.history_selected = Some(id.to_owned());
                cx.notify();
            })
            .ok();
        }));

        div()
            .id("gallery-scroll")
            .flex()
            .flex_col()
            .size_full()
            .bg(tokens.background())
            .text_color(tokens.text())
            .p_6()
            .gap_4()
            .overflow_y_scroll()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("sofui component gallery"),
            )
            .child(div().text_xs().text_color(tokens.text_muted()).child(
                "Components are consumed from sofui; the gallery does not import the application.",
            ))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::primary_with_id("gallery.copy.primary", "Copy sample")
                            .focus_handle(self.primary_focus.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    this.field.text(cx),
                                ));
                                this.copied = true;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::with_id("gallery.copy.secondary", "Copy sample")
                            .focus_handle(self.secondary_focus.clone())
                            .on_click(view_click(cx, |this, _window, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    this.input.text(cx),
                                ));
                                this.copied = true;
                                cx.notify();
                            })),
                    )
                    .child(Button::with_id("gallery.disabled", "Disabled").disabled(true))
                    .child(
                        Button::with_id("gallery.selected", "Selected")
                            .variant(ButtonVariant::Primary)
                            .focus_handle(self.variant_focus.clone())
                            .on_click(view_click(cx, |_this, _window, _cx| {})),
                    )
                    .child(copy_feedback(self.copied, "Copied")),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::with_id(
                            "gallery.theme",
                            format!("Theme: {}", active_theme().label()),
                        )
                        .focus_handle(self.theme_focus.clone())
                        .on_click(view_click(cx, |_this, _window, cx| {
                            let next = match active_theme() {
                                ThemeVariant::Graphite => ThemeVariant::CatppuccinFrappe,
                                ThemeVariant::CatppuccinFrappe => ThemeVariant::Graphite,
                            };
                            set_active_theme(next);
                            cx.notify();
                        })),
                    )
                    .child(
                        HoldButton::new("gallery-clear", "Clear (hold 1s)")
                            .progress(self.hold_progress)
                            .focus_handle(self.hold_focus.clone())
                            .on_press(view_click(cx, |this, _window, cx| this.begin_hold(cx)))
                            .on_release(view_click(cx, |this, _window, cx| this.cancel_hold(cx)))
                            .on_keyboard(view_click(cx, |this, _window, cx| {
                                this.confirm_visible = true;
                                cx.notify();
                            })),
                    )
                    .when(self.confirmed, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(tokens.text_muted())
                                .child("Hold completed."),
                        )
                    }),
            )
            .when(self.confirm_visible, |this| {
                this.child(
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
                        .child(div().text_xs().child("Confirm destructive action?"))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_2()
                                .child(
                                    Button::primary_with_id("gallery.confirm", "Confirm")
                                        .focus_handle(self.confirm_focus.clone())
                                        .on_click(view_click(cx, |this, _window, cx| {
                                            this.confirmed = true;
                                            this.confirm_visible = false;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::with_id("gallery.cancel", "Cancel")
                                        .focus_handle(self.cancel_focus.clone())
                                        .on_click(view_click(cx, |this, _window, cx| {
                                            this.confirm_visible = false;
                                            cx.notify();
                                        })),
                                ),
                        ),
                )
            })
            .child(
                div().text_xs().text_color(tokens.text_muted()).child(
                    "Tab/Shift-Tab move focus; Enter or Space activates the focused control.",
                ),
            )
            .child(
                div().w_full().child(
                    LabeledField::new("Single-line field", self.field.render("gallery.field"))
                        .hint("normal and focused"),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .h_48()
                    .flex_shrink_0()
                    .child(panel(
                        "Input",
                        "multiline editor",
                        self.input.render(false, "gallery.input"),
                    ))
                    .child(panel(
                        "Result",
                        "readonly, selectable",
                        self.result.render(true, "gallery.result"),
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .child(div().flex_1().child(diagnostic_banner(
                        DiagnosticSeverity::Error,
                        "Unexpected token at the end of the document.",
                        Some((3, 8)),
                    )))
                    .child(div().flex_1().child(diagnostic_banner(
                        DiagnosticSeverity::Warning,
                        "Large input is not truncated silently.",
                        None,
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .h_48()
                    .flex_shrink_0()
                    .child(div().flex_1().min_h_0().child(panel(
                        "History list",
                        "select + unavailable",
                        history_panel,
                    )))
                    .child(div().flex_1().min_h_0().child(panel(
                        "Empty state",
                        "neutral",
                        empty_state("Nothing to show yet"),
                    ))),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        init(cx);
        set_dark_theme(None, cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| Gallery::new(window, cx));
            mount(view, window, cx)
        })
        .expect("open gallery window");
    });
}
