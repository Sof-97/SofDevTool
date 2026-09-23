//! Executable gallery for the owner-maintained component library.
//!
//! It demonstrates the actual exported components in their normal, focused,
//! disabled, invalid and empty states, plus a generic selectable list, hold
//! action, confirmation and theme switch used by the application. It builds
//! without the SofDevTool application crate.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, App, ClipboardItem, Context, FocusHandle, IntoElement, Render, Subscription, Window,
    WindowOptions,
};
use sofui::{
    active_theme, apply_custom_theme, apply_theme, copy_feedback, diagnostic_banner, init, mount,
    panel, view_click, Button, ButtonVariant, ConfirmationBar, DiagnosticSeverity, HoldButton,
    HoldController, LabeledField, NumericStepper, SelectableList, SelectableListFocus,
    SelectableRow, TextEditor, TextField, ThemePalette, ThemeTokens, ThemeVariant,
};

// A gallery-only custom palette with distinct focus and diagnostic colors.
const GALLERY_CUSTOM: ThemePalette = ThemePalette {
    background: 0x171c1a,
    surface: 0x202a26,
    surface_raised: 0x2a3831,
    border: 0x52665a,
    text: 0xf0f4ed,
    text_muted: 0xb4c3b7,
    accent: 0x8ae0b0,
    accent_text: 0x102218,
    danger: 0xff9a9e,
    warning: 0xf5ca79,
};

struct Gallery {
    field: TextField,
    input: TextEditor,
    result: TextEditor,
    primary_focus: FocusHandle,
    secondary_focus: FocusHandle,
    variant_focus: FocusHandle,
    theme_focus: FocusHandle,
    custom_focus: FocusHandle,
    hold_focus: FocusHandle,
    confirm_focus: FocusHandle,
    cancel_focus: FocusHandle,
    channel_focus: [FocusHandle; 2],
    channel_value: i32,
    copied: bool,
    list_selected: Option<String>,
    list_items: Vec<SelectableRow>,
    list_focus: SelectableListFocus,
    hold: HoldController,
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
        let list_items = vec![
            SelectableRow {
                id: "a".into(),
                label: "2026-09-22T10:00:00Z".into(),
                preview: "SHA-256 · ba7816bf8f01cfea…".into(),
                status: None,
                selectable: true,
            },
            SelectableRow {
                id: "b".into(),
                label: "2026-09-22T09:59:00Z".into(),
                preview: "Unavailable snapshot".into(),
                status: Some("Unavailable".into()),
                selectable: true,
            },
            SelectableRow {
                id: "c".into(),
                label: "2026-09-22T09:58:00Z".into(),
                preview: "Failed to load".into(),
                status: Some("Failed".into()),
                selectable: false,
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
            custom_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            hold_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            confirm_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            cancel_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            channel_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true).tab_index(0)),
            channel_value: 250,
            copied: false,
            list_selected: Some("a".into()),
            list_items,
            list_focus: SelectableListFocus::new(),
            hold: HoldController::new(),
            confirm_visible: false,
            confirmed: false,
            _subscriptions: subscriptions,
        }
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let selected = self.list_selected.clone();
        let weak = cx.weak_entity();
        let list = SelectableList::new(
            "gallery.list",
            "Sample records",
            self.list_items.clone(),
            selected,
            "No sample records.",
            self.list_focus.clone(),
        )
        .summary("3 items")
        .on_select(std::rc::Rc::new(move |id, _window, cx| {
            weak.update(cx, |this, cx| {
                this.list_selected = Some(id.to_owned());
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
                            apply_theme(next, cx);
                        })),
                    )
                    .child(
                        Button::with_id(
                            "gallery.custom-theme",
                            if ThemeTokens::active().palette() == GALLERY_CUSTOM {
                                "Custom palette: on"
                            } else {
                                "Custom palette"
                            },
                        )
                        .focus_handle(self.custom_focus.clone())
                        .on_click(view_click(cx, |_this, _window, cx| {
                            apply_custom_theme(ThemeTokens::from_palette(GALLERY_CUSTOM), cx);
                        })),
                    )
                    .child(
                        HoldButton::new("gallery-clear", "Clear (hold 1s)", self.hold.clone())
                            .duration(Duration::from_secs(1))
                            .focus_handle(self.hold_focus.clone())
                            .on_complete({
                                let weak = cx.weak_entity();
                                move |cx| {
                                    weak.update(cx, |this, cx| {
                                        this.confirmed = true;
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            })
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
                    ConfirmationBar::new(
                        "gallery.confirmation",
                        "Confirm destructive action?",
                        "Confirm",
                        "Cancel",
                    )
                    .focus_handles(self.confirm_focus.clone(), self.cancel_focus.clone())
                    .on_confirm(view_click(cx, |this, _window, cx| {
                        this.confirmed = true;
                        this.confirm_visible = false;
                        cx.notify();
                    }))
                    .on_cancel(view_click(cx, |this, _window, cx| {
                        this.confirm_visible = false;
                        cx.notify();
                    })),
                )
            })
            .child(
                div().text_xs().text_color(tokens.text_muted()).child(
                    "Tab/Shift-Tab move focus; Enter or Space activates the focused control.",
                ),
            )
            .child(
                NumericStepper::new(
                    "gallery.numeric",
                    "Sample channel",
                    Some(self.channel_value),
                    0,
                    255,
                    5,
                )
                .focus_handles(self.channel_focus[0].clone(), self.channel_focus[1].clone())
                .on_step({
                    let weak = cx.weak_entity();
                    move |delta, _window, cx| {
                        weak.update(cx, |this, cx| {
                            this.channel_value =
                                NumericStepper::stepped(this.channel_value, delta, 0, 255);
                            cx.notify();
                        })
                        .ok();
                    }
                }),
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
                        "Selectable list",
                        "select + unavailable",
                        list,
                    )))
                    .child(div().flex_1().min_h_0().child(panel(
                        "Empty state",
                        "neutral",
                        SelectableList::new(
                            "gallery.empty-list",
                            "Empty records",
                            vec![],
                            None,
                            "Nothing to show yet",
                            SelectableListFocus::new(),
                        ),
                    ))),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        init(cx);
        apply_theme(ThemeVariant::Graphite, cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| Gallery::new(window, cx));
            mount(view, window, cx)
        })
        .expect("open gallery window");
    });
}
