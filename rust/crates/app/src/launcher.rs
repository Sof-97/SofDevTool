//! A transient, nonactivating Utility Launcher window.

use gpui::prelude::*;
use gpui::{
    div, point, px, size, AnyWindowHandle, App, Context, DisplayId, IntoElement, Render, Window,
    WindowBounds, WindowKind, WindowOptions,
};
use sofdevtool_ui::{mount, view_click, Button, LabeledField, TextField, ThemeTokens};

use crate::registry::{OpenUtility, UtilityId, UtilityRegistry};
use crate::workbench::Workbench;

gpui::actions!(
    launcher_actions,
    [DismissLauncher, SelectPrevious, SelectNext, OpenSelection]
);

/// Registers Launcher-only dismissal handling. The action is scoped to the
/// popup view, so Escape does not alter Workbench text editing.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("escape", DismissLauncher, Some("SofDevToolLauncher")),
        gpui::KeyBinding::new("up", SelectPrevious, Some("SofDevToolLauncher")),
        gpui::KeyBinding::new("down", SelectNext, Some("SofDevToolLauncher")),
        gpui::KeyBinding::new("enter", OpenSelection, Some("SofDevToolLauncher")),
    ]);
}

/// Opens a native GPUI popup rather than an in-window palette. GPUI builds it as
/// a nonactivating `NSPanel`, so `focus: true` makes the panel key for keyboard
/// input without activating the owning application.
pub fn show(
    registry: UtilityRegistry,
    main_window: AnyWindowHandle,
    workbench: gpui::WeakEntity<Workbench>,
    cx: &mut App,
) -> Option<AnyWindowHandle> {
    let display_id = active_display(cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(gpui::Bounds::centered(
            display_id,
            size(px(580.), px(430.)),
            cx,
        ))),
        display_id,
        focus: true,
        kind: WindowKind::PopUp,
        is_resizable: false,
        is_minimizable: false,
        titlebar: None,
        ..Default::default()
    };
    cx.open_window(options, move |window, cx| {
        let dismiss_workbench = workbench.clone();
        window.on_window_should_close(cx, move |_window, cx| {
            let _ =
                dismiss_workbench.update(cx, |workbench, cx| workbench.launcher_did_dismiss(cx));
            true
        });
        let view = cx.new(|cx| LauncherView::new(window, cx, registry, main_window, workbench));
        let focus_view = view.clone();
        let root = mount(view, window, cx);
        crate::native_window::configure_launcher_panel(window);
        crate::native_window::show_nonactivating(window);
        // The component Root installs the input registry when it is mounted.
        // Focusing before that boundary exists aborts in GPUI's macOS event
        // bridge, so request focus on the next application turn.
        window.defer(cx, move |window, cx| {
            crate::native_window::focus_launcher(window);
            focus_view.update(cx, |view, cx| view.focus_search(window, cx));
        });
        root
    })
    .ok()
    .map(Into::into)
}

fn active_display(cx: &App) -> Option<DisplayId> {
    let (x, y) = crate::native_window::mouse_location()?;
    let location = point(px(x), px(y));
    cx.displays()
        .into_iter()
        .find(|display| display.bounds().contains(&location))
        .map(|display| display.id())
}

pub struct LauncherView {
    registry: UtilityRegistry,
    search: TextField,
    selected: Option<UtilityId>,
    main_window: AnyWindowHandle,
    workbench: gpui::WeakEntity<Workbench>,
    was_active: bool,
    _search_subscription: gpui::Subscription,
    _activation_subscription: gpui::Subscription,
}

impl LauncherView {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        registry: UtilityRegistry,
        main_window: AnyWindowHandle,
        workbench: gpui::WeakEntity<Workbench>,
    ) -> Self {
        let search = TextField::new(window, cx);
        let selected = registry.search("").first().copied();
        let search_subscription = search.on_change_in(window, cx, |this, _window, cx| {
            this.selected = this.results(cx).first().copied();
            cx.notify();
        });
        // A nonactivating panel receives keys while it is the key window. Losing
        // key status (a click in another window or application) dismisses it,
        // which is the click-away behavior. The callback fires once immediately
        // with the current state, so the first non-active result is ignored.
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            let active = window.is_window_active();
            if this.was_active && !active {
                this.was_active = false;
                this.dismiss(window, cx);
            } else {
                this.was_active = active;
            }
        });
        Self {
            registry,
            search,
            selected,
            main_window,
            workbench,
            was_active: window.is_window_active(),
            _search_subscription: search_subscription,
            _activation_subscription: activation_subscription,
        }
    }

    fn results(&self, cx: &App) -> Vec<UtilityId> {
        self.registry.search(&self.search.text(cx))
    }

    fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.focus(window, cx);
    }

    fn open(&mut self, id: UtilityId, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .workbench
            .update(cx, |workbench, cx| workbench.open(OpenUtility(id), cx))
            .unwrap_or(false)
        {
            let main_window = self.main_window;
            let _ = main_window.update(cx, |_, window, _| window.activate_window());
        }
        self.dismiss(window, cx);
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self
            .workbench
            .update(cx, |workbench, cx| workbench.launcher_did_dismiss(cx));
        window.remove_window();
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let results = self.results(cx);
        if results.is_empty() {
            self.selected = None;
        } else {
            let index = self
                .selected
                .and_then(|selected| results.iter().position(|id| *id == selected))
                .unwrap_or(0) as isize;
            let next = (index + delta).rem_euclid(results.len() as isize) as usize;
            self.selected = Some(results[next]);
        }
        cx.notify();
    }

    fn dismiss_action(&mut self, _: &DismissLauncher, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }

    fn select_previous(&mut self, _: &SelectPrevious, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(-1, cx);
    }

    fn select_next(&mut self, _: &SelectNext, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selection(1, cx);
    }

    fn open_selection(&mut self, _: &OpenSelection, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.selected {
            self.open(id, window, cx);
        }
    }
}

impl Render for LauncherView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = ThemeTokens::active();
        let results = self.results(cx);
        let mut list = div().flex().flex_col().gap_2().flex_1().min_h_0();
        for id in results {
            let definition = self
                .registry
                .definition(id)
                .expect("searched definition exists");
            let selected = self.selected == Some(id);
            list = list.child(
                Button::new(definition.name)
                    .variant(if selected {
                        sofdevtool_ui::ButtonVariant::Primary
                    } else {
                        sofdevtool_ui::ButtonVariant::Secondary
                    })
                    .on_click(view_click(cx, move |this, window, cx| {
                        this.open(id, window, cx);
                    })),
            );
        }
        div()
            .flex()
            .flex_col()
            .size_full()
            .key_context("SofDevToolLauncher")
            .on_action(cx.listener(Self::dismiss_action))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::open_selection))
            .p_5()
            .gap_4()
            .bg(tokens.surface())
            .text_color(tokens.text())
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Find a Utility"),
            )
            .child(LabeledField::new(
                "Search",
                self.search.render("launcher.search"),
            ))
            .child(list)
            .child(
                div()
                    .text_xs()
                    .text_color(tokens.text_muted())
                    .child("Escape dismisses this launcher. Selection opens the Workbench."),
            )
    }
}
