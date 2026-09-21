//! Ticket 02 native proof application.
//!
//! This composes only the two concrete workspaces needed for the migration
//! check. Launcher, shortcuts, lifecycle and history remain outside its scope.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, App, Context, FocusHandle, Menu, MenuItem, OsAction, Render, TitlebarOptions, Window,
    WindowOptions,
};
use sofdevtool_app::clipboard::{Clipboard, GpuiClipboard};
use sofdevtool_app::identity;
use sofdevtool_app::json_workspace::JsonWorkspace;
use sofdevtool_app::text_diff::TextDiffWorkspace;
use sofdevtool_ui::{
    init, mount, panel, run, set_dark_theme, view_click, Button, ButtonVariant, ThemeTokens,
};

gpui::actions!(ticket02_actions, [Quit, Copy, Paste]);
#[cfg(debug_assertions)]
gpui::actions!(ticket02_debug_actions, [SimulateRendererFailure]);

fn window_options() -> WindowOptions {
    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(identity::APP_DISPLAY_NAME.into()),
            ..Default::default()
        }),
        app_id: Some(identity::BUNDLE_IDENTIFIER.to_string()),
        ..Default::default()
    }
}

struct Ticket02Workbench {
    json: gpui::Entity<JsonWorkspace>,
    text_diff: gpui::Entity<TextDiffWorkspace>,
    text_diff_selected: bool,
    json_focus: FocusHandle,
    text_diff_focus: FocusHandle,
}

impl Ticket02Workbench {
    fn new(window: &mut Window, cx: &mut Context<Self>, clipboard: Rc<dyn Clipboard>) -> Self {
        let json = cx.new(|cx| JsonWorkspace::new(window, cx, clipboard.clone()));
        let text_diff = cx.new(|cx| TextDiffWorkspace::new(window, cx, clipboard));
        text_diff.read(cx).set_active(false);
        Self {
            json,
            text_diff,
            text_diff_selected: false,
            json_focus: cx.focus_handle().tab_stop(true).tab_index(0),
            text_diff_focus: cx.focus_handle().tab_stop(true).tab_index(0),
        }
    }

    fn select_json(&mut self, cx: &mut Context<Self>) {
        self.text_diff.read(cx).focus_parent();
        self.text_diff.read(cx).set_active(false);
        self.text_diff_selected = false;
        cx.notify();
    }

    fn select_text_diff(&mut self, cx: &mut Context<Self>) {
        self.text_diff.read(cx).focus_parent();
        self.text_diff.read(cx).set_active(true);
        self.text_diff_selected = true;
        cx.notify();
    }
}

impl Render for Ticket02Workbench {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::graphite();
        let text_diff_selected = self.text_diff_selected;
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(tokens.background())
            .text_color(tokens.text())
            .child(
                div()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(tokens.border())
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Developer Toolbox"),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .w_56()
                            .p_3()
                            .gap_2()
                            .border_r_1()
                            .border_color(tokens.border())
                            .child(
                                Button::new("JSON")
                                    .focus_handle(self.json_focus.clone())
                                    .variant(if text_diff_selected {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Primary
                                    })
                                    .on_click(view_click(cx, |this, _window, cx| {
                                        this.select_json(cx)
                                    })),
                            )
                            .child(
                                Button::new("Text Diff")
                                    .focus_handle(self.text_diff_focus.clone())
                                    .variant(if text_diff_selected {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Secondary
                                    })
                                    .on_click(view_click(cx, |this, _window, cx| {
                                        this.select_text_diff(cx)
                                    })),
                            ),
                    )
                    .child(if text_diff_selected {
                        div().m_3().flex().flex_1().min_w_0().min_h_0().child(panel(
                            "Text Diff",
                            "persistent session",
                            div()
                                .flex()
                                .flex_1()
                                .min_h_0()
                                .child(self.text_diff.clone()),
                        ))
                    } else {
                        div().m_3().flex().flex_1().min_w_0().min_h_0().child(panel(
                            "JSON",
                            "persistent session",
                            div().flex().flex_1().min_h_0().child(self.json.clone()),
                        ))
                    }),
            )
    }
}

fn main() {
    #[cfg(debug_assertions)]
    let text_diff_for_diagnostics: Rc<RefCell<Option<gpui::Entity<TextDiffWorkspace>>>> =
        Rc::new(RefCell::new(None));
    run(move |cx: &mut App| {
        init(cx);
        set_dark_theme(None, cx);
        // GPUI uses these bindings to assign Cmd-C/V key equivalents to the
        // native selectors. AppKit then delivers them to WKWebView's first
        // responder, while GPUI inputs retain their own copy/paste routing.
        cx.bind_keys([
            gpui::KeyBinding::new("cmd-q", Quit, None),
            gpui::KeyBinding::new("cmd-c", Copy, None),
            gpui::KeyBinding::new("cmd-v", Paste, None),
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        #[cfg(debug_assertions)]
        {
            let text_diff_for_diagnostics = text_diff_for_diagnostics.clone();
            cx.on_action(move |_: &SimulateRendererFailure, cx| {
                if let Some(text_diff) = text_diff_for_diagnostics.borrow().clone() {
                    text_diff.update(cx, |workspace, cx| {
                        workspace.simulate_renderer_failure(cx);
                    });
                }
            });
        }
        let mut menus = vec![
            Menu::new(identity::APP_DISPLAY_NAME)
                .items([MenuItem::action("Quit SofDevTool", Quit)]),
            Menu::new("Edit").items([
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
            ]),
        ];
        #[cfg(debug_assertions)]
        menus.push(Menu::new("Renderer diagnostics").items([MenuItem::action(
            "Test renderer failure",
            SimulateRendererFailure,
        )]));
        cx.set_menus(menus);
        let clipboard: Rc<dyn Clipboard> = Rc::new(GpuiClipboard);
        #[cfg(debug_assertions)]
        let text_diff_for_diagnostics = text_diff_for_diagnostics.clone();
        cx.open_window(window_options(), move |window, cx| {
            let view = cx.new(|cx| Ticket02Workbench::new(window, cx, clipboard.clone()));
            #[cfg(debug_assertions)]
            {
                *text_diff_for_diagnostics.borrow_mut() = Some(view.read(cx).text_diff.clone());
            }
            mount(view, window, cx)
        })
        .expect("open the Ticket 02 proof window");
    });
}
