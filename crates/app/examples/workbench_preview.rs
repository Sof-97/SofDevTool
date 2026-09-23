//! Ticket 11 review surface: a single native Workbench with synthetic state.
//!
//! Run with `cargo run -p sofdevtool-app --example workbench_preview`. This is
//! deliberately separate from application startup, preferences and History.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, px, rgb, size, App, Bounds, ClipboardItem, Context, FocusHandle, IntoElement, Render,
    Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use sofdevtool_ui::{
    active_theme, apply_theme, copy_feedback, diagnostic_banner, init, mount, panel, view_click,
    Button, ButtonVariant, ConfirmationBar, DiagnosticSeverity, HoldButton, HoldController,
    NumericStepper, SelectableList, SelectableListFocus, SelectableRow, TextEditor, TextField,
    ThemeTokens, ThemeVariant,
};

struct CatalogItem {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    glyph: &'static str,
}

const CATALOG: [CatalogItem; 15] = [
    CatalogItem {
        id: "json",
        name: "JSON",
        category: "FORMAT & CONVERT",
        glyph: "{}",
    },
    CatalogItem {
        id: "yaml",
        name: "YAML / JSON",
        category: "FORMAT & CONVERT",
        glyph: "⇄",
    },
    CatalogItem {
        id: "base64",
        name: "Base64",
        category: "FORMAT & CONVERT",
        glyph: "64",
    },
    CatalogItem {
        id: "url",
        name: "URL Encoding",
        category: "FORMAT & CONVERT",
        glyph: "%",
    },
    CatalogItem {
        id: "color",
        name: "Color Conversion",
        category: "FORMAT & CONVERT",
        glyph: "◐",
    },
    CatalogItem {
        id: "hashes",
        name: "Hashes",
        category: "ENCODE & INSPECT",
        glyph: "#",
    },
    CatalogItem {
        id: "jwt",
        name: "JWT Decoder",
        category: "ENCODE & INSPECT",
        glyph: "◈",
    },
    CatalogItem {
        id: "timestamps",
        name: "Timestamps",
        category: "ENCODE & INSPECT",
        glyph: "◷",
    },
    CatalogItem {
        id: "identifiers",
        name: "Identifier Generator",
        category: "GENERATE",
        glyph: "ID",
    },
    CatalogItem {
        id: "random",
        name: "Random String",
        category: "GENERATE",
        glyph: "✳",
    },
    CatalogItem {
        id: "sample",
        name: "Sample Data",
        category: "GENERATE",
        glyph: "▦",
    },
    CatalogItem {
        id: "regex",
        name: "Regex",
        category: "COMPARE & TEST",
        glyph: ".*",
    },
    CatalogItem {
        id: "diff",
        name: "Text Diff",
        category: "COMPARE & TEST",
        glyph: "±",
    },
    CatalogItem {
        id: "case",
        name: "Case Conversion",
        category: "COMPARE & TEST",
        glyph: "Aa",
    },
    CatalogItem {
        id: "space",
        name: "Whitespace",
        category: "COMPARE & TEST",
        glyph: "¶",
    },
];

const SAMPLE_INPUT: &str =
    "{\"name\":\"SofDevTool\",\"version\":1,\"features\":[\"native\",\"offline\"]}";
const SAMPLE_RESULT: &str = "{\n  \"name\": \"SofDevTool\",\n  \"version\": 1,\n  \"features\": [\n    \"native\",\n    \"offline\"\n  ]\n}";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Library,
    Recent,
    Favorites,
}

struct Preview {
    search: TextField,
    input: TextEditor,
    result: TextEditor,
    selected: usize,
    scope: Scope,
    favorites: Vec<&'static str>,
    recents: Vec<&'static str>,
    history_open: bool,
    history_rows: Vec<SelectableRow>,
    history_selected: Option<String>,
    history_focus: SelectableListFocus,
    hold: HoldController,
    confirm_restore: bool,
    confirm_clear: bool,
    invalid: bool,
    copied: bool,
    channels: [i32; 3],
    interaction_count: usize,
    shell_notice: Option<&'static str>,
    focus_scope: [FocusHandle; 3],
    focus_catalog: Vec<FocusHandle>,
    focus_theme: [FocusHandle; 2],
    focus_launcher: FocusHandle,
    focus_settings: FocusHandle,
    focus_history: FocusHandle,
    focus_favorite: FocusHandle,
    focus_format: FocusHandle,
    focus_invalid: FocusHandle,
    focus_copy: FocusHandle,
    focus_restore: FocusHandle,
    focus_hold: FocusHandle,
    focus_confirm: FocusHandle,
    focus_cancel: FocusHandle,
    focus_channels: [[FocusHandle; 2]; 3],
    _subscriptions: Vec<Subscription>,
}

impl Preview {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = TextField::new(window, cx);
        let input = TextEditor::new(window, cx);
        let result = TextEditor::new(window, cx);
        input.assign_text(SAMPLE_INPUT, window, cx);
        result.assign_text(SAMPLE_RESULT, window, cx);
        let subscriptions = vec![
            search.on_change_in(window, cx, |_this, _window, cx| cx.notify()),
            input.on_change_in(window, cx, |this, window, cx| {
                let source = this.input.text(cx);
                if source.trim().is_empty() {
                    this.invalid = false;
                    this.result.assign_text("", window, cx);
                } else {
                    match serde_json::from_str::<serde_json::Value>(&source) {
                        Ok(value) => {
                            this.invalid = false;
                            this.result.assign_text(
                                serde_json::to_string_pretty(&value).unwrap_or_default(),
                                window,
                                cx,
                            );
                        }
                        Err(_) => {
                            this.invalid = true;
                            this.result.assign_text("", window, cx);
                        }
                    }
                }
                this.copied = false;
                cx.notify();
            }),
        ];
        let focus = |cx: &mut Context<Self>| cx.focus_handle().tab_stop(true).tab_index(0);
        Self {
            search,
            input,
            result,
            selected: 0,
            scope: Scope::Library,
            favorites: vec!["json", "regex", "color"],
            recents: vec!["json", "color", "regex", "base64"],
            history_open: true,
            history_rows: vec![
                SelectableRow {
                    id: "h-1".into(),
                    label: "Today · 10:42".into(),
                    preview: "SofDevTool · 2 fields".into(),
                    status: None,
                    selectable: true,
                },
                SelectableRow {
                    id: "h-2".into(),
                    label: "Today · 09:18".into(),
                    preview: "config.json · 5 fields".into(),
                    status: None,
                    selectable: true,
                },
                SelectableRow {
                    id: "h-3".into(),
                    label: "Yesterday · 17:05".into(),
                    preview: "Unavailable snapshot".into(),
                    status: Some("Unavailable".into()),
                    selectable: true,
                },
            ],
            history_selected: Some("h-1".into()),
            history_focus: SelectableListFocus::new(),
            hold: HoldController::new(),
            confirm_restore: false,
            confirm_clear: false,
            invalid: false,
            copied: false,
            channels: [91, 141, 239],
            interaction_count: 0,
            shell_notice: None,
            focus_scope: std::array::from_fn(|_| focus(cx)),
            focus_catalog: (0..CATALOG.len()).map(|_| focus(cx)).collect(),
            focus_theme: std::array::from_fn(|_| focus(cx)),
            focus_launcher: focus(cx),
            focus_settings: focus(cx),
            focus_history: focus(cx),
            focus_favorite: focus(cx),
            focus_format: focus(cx),
            focus_invalid: focus(cx),
            focus_copy: focus(cx),
            focus_restore: focus(cx),
            focus_hold: focus(cx),
            focus_confirm: focus(cx),
            focus_cancel: focus(cx),
            focus_channels: std::array::from_fn(|_| std::array::from_fn(|_| focus(cx))),
            _subscriptions: subscriptions,
        }
    }

    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected = index;
        let id = CATALOG[index].id;
        self.recents.retain(|recent| *recent != id);
        self.recents.insert(0, id);
        self.recents.truncate(5);
        self.interaction_count += 1;
        cx.notify();
    }

    fn clear_history(&mut self, cx: &mut Context<Self>) {
        self.history_rows.clear();
        self.history_selected = None;
        self.confirm_clear = false;
        self.interaction_count += 1;
        cx.notify();
    }

    fn restore_sample(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.assign_text(SAMPLE_INPUT, window, cx);
        self.result.assign_text(SAMPLE_RESULT, window, cx);
        self.invalid = false;
        self.confirm_restore = false;
        self.interaction_count += 1;
        cx.notify();
    }
}

impl Render for Preview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = ThemeTokens::active();
        let selected = self.selected;
        let current = &CATALOG[selected];
        let query = self.search.text(cx).to_lowercase();
        let mut catalog = div()
            .id("preview.catalog.list")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .overflow_y_scroll();
        let mut category = "";
        let mut shown = 0;
        for (index, item) in CATALOG.iter().enumerate() {
            let in_scope = match self.scope {
                Scope::Library => true,
                Scope::Recent => self.recents.contains(&item.id),
                Scope::Favorites => self.favorites.contains(&item.id),
            };
            if !in_scope || !item.name.to_lowercase().contains(&query) {
                continue;
            }
            shown += 1;
            if self.scope == Scope::Library && category != item.category {
                category = item.category;
                catalog = catalog.child(
                    div()
                        .mt_3()
                        .mb_1()
                        .px_2()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(t.text_muted())
                        .child(category),
                );
            }
            let selected_row = index == selected;
            catalog = catalog.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .w_full()
                    .child(
                        div()
                            .w_6()
                            .text_xs()
                            .text_color(t.text_muted())
                            .child(item.glyph),
                    )
                    .child(
                        div().flex_1().min_w_0().child(
                            Button::with_id(format!("preview.catalog.{}", item.id), item.name)
                                .variant(if selected_row {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Secondary
                                })
                                .focus_handle(self.focus_catalog[index].clone())
                                .on_click(view_click(cx, move |this, _, cx| {
                                    this.select(index, cx)
                                })),
                        ),
                    ),
            );
        }
        if shown == 0 {
            catalog = catalog.child(
                div()
                    .p_3()
                    .text_xs()
                    .text_color(t.text_muted())
                    .child("No Utilities match this view."),
            );
        }

        let scope = |label: &'static str, value: Scope, index: usize, cx: &mut Context<Self>| {
            Button::with_id(format!("preview.scope.{index}"), label)
                .variant(if self.scope == value {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                })
                .focus_handle(self.focus_scope[index].clone())
                .on_click(view_click(cx, move |this, _, cx| {
                    this.scope = value;
                    cx.notify();
                }))
        };
        let sidebar = div()
            .flex()
            .flex_col()
            .w(px(252.))
            .min_h_0()
            .flex_shrink_0()
            .bg(t.surface())
            .border_r_1()
            .border_color(t.border())
            .p_3()
            .gap_3()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(scope("Library", Scope::Library, 0, cx))
                    .child(scope("Recent", Scope::Recent, 1, cx))
                    .child(scope("Favorites", Scope::Favorites, 2, cx)),
            )
            .child(self.search.render("preview.search"))
            .child(catalog)
            .child(
                div()
                    .pt_2()
                    .border_t_1()
                    .border_color(t.border())
                    .text_xs()
                    .text_color(t.text_muted())
                    .child(format!("{shown} of 15 Utilities")),
            );

        let focused_status = div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().size_2().rounded_full().bg(t.accent()))
            .child(
                div()
                    .text_xs()
                    .text_color(t.text_muted())
                    .child("LOCAL · OFFLINE"),
            );
        let topbar = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .h(px(54.))
            .flex_shrink_0()
            .px_4()
            .border_b_1()
            .border_color(t.border())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("SofDevTool"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_muted())
                            .child("/ Workbench"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(focused_status)
                    .child(
                        Button::with_id("preview.launcher", "Launcher · ⌃⌥Space")
                            .focus_handle(self.focus_launcher.clone())
                            .on_click(view_click(cx, |this, _, cx| {
                                this.shell_notice =
                                    Some("Launcher opens separately in the application.");
                                this.interaction_count += 1;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::with_id("preview.settings", "Settings")
                            .focus_handle(self.focus_settings.clone())
                            .on_click(view_click(cx, |this, _, cx| {
                                this.shell_notice =
                                    Some("Settings opens separately in the application.");
                                this.interaction_count += 1;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::with_id("preview.theme.graphite", "Graphite")
                            .variant(if active_theme() == ThemeVariant::Graphite {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Secondary
                            })
                            .focus_handle(self.focus_theme[0].clone())
                            .on_click(view_click(cx, |_, _, cx| {
                                apply_theme(ThemeVariant::Graphite, cx)
                            })),
                    )
                    .child(
                        Button::with_id("preview.theme.frappe", "Frappé")
                            .variant(if active_theme() == ThemeVariant::CatppuccinFrappe {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Secondary
                            })
                            .focus_handle(self.focus_theme[1].clone())
                            .on_click(view_click(cx, |_, _, cx| {
                                apply_theme(ThemeVariant::CatppuccinFrappe, cx)
                            })),
                    ),
            );

        let is_favorite = self.favorites.contains(&current.id);
        let heading = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(current.name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_muted())
                            .child("A focused workspace · session stays open while navigating"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::with_id(
                            "preview.favorite",
                            if is_favorite { "★ Saved" } else { "☆ Save" },
                        )
                        .focus_handle(self.focus_favorite.clone())
                        .on_click(view_click(cx, move |this, _, cx| {
                            if this.favorites.contains(&current.id) {
                                this.favorites.retain(|id| *id != current.id);
                            } else {
                                this.favorites.push(current.id);
                            }
                            cx.notify();
                        })),
                    )
                    .child(
                        Button::with_id(
                            "preview.history.toggle",
                            if self.history_open {
                                "Hide History"
                            } else {
                                "Show History"
                            },
                        )
                        .focus_handle(self.focus_history.clone())
                        .on_click(view_click(cx, |this, _, cx| {
                            this.history_open = !this.history_open;
                            cx.notify();
                        })),
                    ),
            );

        let controls = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::primary_with_id("preview.format", "Format JSON")
                    .disabled(selected != 0 || self.invalid)
                    .focus_handle(self.focus_format.clone())
                    .on_click(view_click(cx, |this, window, cx| {
                        let source = this.input.text(cx);
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&source) {
                            this.result.assign_text(
                                serde_json::to_string_pretty(&value).unwrap_or_default(),
                                window,
                                cx,
                            );
                            this.interaction_count += 1;
                            cx.notify();
                        }
                    })),
            )
            .child(
                Button::with_id(
                    "preview.invalid",
                    if self.invalid {
                        "Use valid sample"
                    } else {
                        "Show invalid state"
                    },
                )
                .focus_handle(self.focus_invalid.clone())
                .on_click(view_click(cx, |this, window, cx| {
                    let sample = if this.invalid {
                        SAMPLE_INPUT
                    } else {
                        "{\"name\": }"
                    };
                    this.input.edit_text(sample, window, cx);
                })),
            )
            .child(
                Button::with_id(
                    "preview.copy",
                    if self.copied { "Copied" } else { "Copy result" },
                )
                .disabled(self.invalid || self.result.text(cx).is_empty())
                .focus_handle(self.focus_copy.clone())
                .on_click(view_click(cx, |this, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(this.result.text(cx)));
                    this.copied = true;
                    this.interaction_count += 1;
                    cx.notify();
                })),
            )
            .child(copy_feedback(self.copied, "Preview copy acknowledged"));

        let editors = div()
            .flex()
            .flex_1()
            .min_h_0()
            .gap_3()
            .child(panel(
                "INPUT",
                "editable · JSON sample",
                self.input.render(false, "preview.input"),
            ))
            .child(panel(
                "RESULT",
                if self.invalid {
                    "invalid"
                } else {
                    "read-only · selectable"
                },
                self.result.render(true, "preview.result"),
            ));
        let diagnostic = if self.invalid {
            diagnostic_banner(
                DiagnosticSeverity::Error,
                "Expected a JSON value after the colon.",
                Some((1, 10)),
            )
            .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_xs()
                .text_color(t.text_muted())
                .child("✓ Valid JSON · 2 spaces · keys preserved")
                .into_any_element()
        };

        let swatch_color = rgb(((self.channels[0] as u32) << 16)
            | ((self.channels[1] as u32) << 8)
            | self.channels[2] as u32);
        let mut channel_strip = div().flex().items_center().gap_3();
        for (index, label) in ["R", "G", "B"].into_iter().enumerate() {
            channel_strip = channel_strip.child(
                NumericStepper::new(
                    format!("preview.color.{label}"),
                    label,
                    Some(self.channels[index]),
                    0,
                    255,
                    5,
                )
                .focus_handles(
                    self.focus_channels[index][0].clone(),
                    self.focus_channels[index][1].clone(),
                )
                .on_step({
                    let weak = cx.weak_entity();
                    move |delta, _, cx| {
                        weak.update(cx, |this, cx| {
                            this.channels[index] =
                                NumericStepper::stepped(this.channels[index], delta, 0, 255);
                            this.interaction_count += 1;
                            cx.notify();
                        })
                        .ok();
                    }
                }),
            );
        }
        let color_sample = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(t.border())
            .bg(t.surface())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size_5().rounded_md().bg(swatch_color))
                    .child(div().text_xs().text_color(t.text_muted()).child(format!(
                        "#{:02X}{:02X}{:02X}",
                        self.channels[0], self.channels[1], self.channels[2]
                    ))),
            )
            .child(channel_strip);

        let workspace = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .p_4()
            .gap_3()
            .child(heading)
            .child(div().h(px(1.)).bg(t.border()))
            .child(controls)
            .child(editors)
            .child(diagnostic)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(t.text_muted())
                            .child("COLOR CONTROL · LIVE CHANNELS"),
                    )
                    .child(color_sample),
            );

        let weak = cx.weak_entity();
        let history = SelectableList::new(
            "preview.history",
            "History",
            self.history_rows.clone(),
            self.history_selected.clone(),
            "No preview records",
            self.history_focus.clone(),
        )
        .summary(format!("{} / 25", self.history_rows.len()))
        .on_select(Rc::new(move |id, _, cx| {
            weak.update(cx, |this, cx| {
                this.history_selected = Some(id.to_owned());
                cx.notify();
            })
            .ok();
        }));
        let inspector = div()
            .flex()
            .flex_col()
            .w(px(254.))
            .min_h_0()
            .flex_shrink_0()
            .border_l_1()
            .border_color(t.border())
            .bg(t.surface())
            .p_3()
            .gap_3()
            .child(
                div()
                    .text_xs()
                    .text_color(t.text_muted())
                    .child("JSON · newest first · synthetic data"),
            )
            .child(history)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::primary_with_id("preview.restore", "Restore selected")
                            .disabled(self.history_selected.is_none())
                            .focus_handle(self.focus_restore.clone())
                            .on_click(view_click(cx, |this, _, cx| {
                                this.confirm_restore = true;
                                cx.notify();
                            })),
                    )
                    .child(
                        HoldButton::new("preview.clear", "Clear · hold 1s", self.hold.clone())
                            .duration(Duration::from_secs(1))
                            .disabled(self.history_rows.is_empty())
                            .focus_handle(self.focus_hold.clone())
                            .on_complete({
                                let weak = cx.weak_entity();
                                move |cx| {
                                    weak.update(cx, |this, cx| this.clear_history(cx)).ok();
                                }
                            })
                            .on_keyboard(view_click(cx, |this, _, cx| {
                                this.confirm_clear = true;
                                cx.notify();
                            })),
                    ),
            );

        let confirmation = if self.confirm_restore {
            Some(
                ConfirmationBar::new(
                    "preview.confirm.restore",
                    "Replace the current sample?",
                    "Restore",
                    "Cancel",
                )
                .focus_handles(self.focus_confirm.clone(), self.focus_cancel.clone())
                .on_confirm(view_click(cx, |this, window, cx| {
                    this.restore_sample(window, cx)
                }))
                .on_cancel(view_click(cx, |this, _, cx| {
                    this.confirm_restore = false;
                    cx.notify();
                })),
            )
        } else if self.confirm_clear {
            Some(
                ConfirmationBar::new(
                    "preview.confirm.clear",
                    "Clear in-memory preview records?",
                    "Clear",
                    "Cancel",
                )
                .focus_handles(self.focus_confirm.clone(), self.focus_cancel.clone())
                .on_confirm(view_click(cx, |this, _, cx| this.clear_history(cx)))
                .on_cancel(view_click(cx, |this, _, cx| {
                    this.confirm_clear = false;
                    cx.notify();
                })),
            )
        } else {
            None
        };
        let footer = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .h(px(37.))
            .flex_shrink_0()
            .px_4()
            .border_t_1()
            .border_color(t.border())
            .text_xs()
            .text_color(t.text_muted())
            .child(
                self.shell_notice
                    .unwrap_or("PREVIEW · no application data or preferences"),
            )
            .child(format!(
                "{} interactions · Tab to focus · resize the window",
                self.interaction_count
            ));

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(t.background())
            .text_color(t.text())
            .text_size(px(13.))
            .child(topbar)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(sidebar)
                    .child(workspace)
                    .when(self.history_open, |this| this.child(inspector)),
            )
            .when_some(confirmation, |this, bar| {
                this.child(div().px_4().py_2().child(bar))
            })
            .child(footer)
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        init(cx);
        apply_theme(ThemeVariant::Graphite, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1340.), px(820.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("SofDevTool · Workbench preview".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| Preview::new(window, cx));
            mount(view, window, cx)
        })
        .expect("open native Workbench preview");
    });
}
