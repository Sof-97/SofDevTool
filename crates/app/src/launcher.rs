//! A transient, nonactivating Utility Launcher window.

use gpui::prelude::*;
use gpui::{
    div, point, px, size, AnyWindowHandle, App, Context, DisplayId, Entity, Focusable, IntoElement,
    Render, ScrollHandle, Subscription, Window, WindowBounds, WindowKind, WindowOptions,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    ActiveTheme as _, Root,
};

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
        let root = cx.new(|cx| Root::new(view, window, cx));
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
    search: Entity<InputState>,
    selected: Option<UtilityId>,
    main_window: AnyWindowHandle,
    workbench: gpui::WeakEntity<Workbench>,
    was_active: bool,
    scroll: ScrollHandle,
    _search_subscription: Subscription,
    _activation_subscription: Subscription,
}

impl LauncherView {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        registry: UtilityRegistry,
        main_window: AnyWindowHandle,
        workbench: gpui::WeakEntity<Workbench>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx));
        let selected = registry.search("").first().copied();
        let search_subscription = cx.subscribe_in(
            &search,
            window,
            |this, _entity, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.selected = this.results(cx).first().copied();
                    this.scroll.scroll_to_item(0);
                    cx.notify();
                }
            },
        );
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
            scroll: ScrollHandle::new(),
            _search_subscription: search_subscription,
            _activation_subscription: activation_subscription,
        }
    }

    fn results(&self, cx: &App) -> Vec<UtilityId> {
        self.registry.search(&self.search.read(cx).value())
    }

    fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.read(cx).focus_handle(cx).focus(window, cx);
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
            self.scroll.scroll_to_item(next);
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
        let theme = cx.theme().clone();
        let results = self.results(cx);
        let result_count = results.len();
        let mut list = div()
            .id("launcher.results")
            .flex()
            .flex_col()
            .gap_1()
            .flex_1()
            .min_h_0()
            .track_scroll(&self.scroll)
            .overflow_y_scroll();
        for id in results {
            let definition = self
                .registry
                .definition(id)
                .expect("searched definition exists");
            let selected = self.selected == Some(id);
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .w_full()
                    .px_1()
                    .py_1()
                    .rounded(theme.radius)
                    .bg(if selected {
                        theme.secondary
                    } else {
                        theme.popover
                    })
                    .child(
                        div().flex_1().min_w_0().child(
                            Button::new(format!("launcher.utility.{}", id.slug()))
                                .label(definition.name)
                                .when(selected, |button| button.primary())
                                .on_click(cx.listener(move |this, _event, window, cx| {
                                    this.open(id, window, cx);
                                })),
                        ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(definition.category),
                    ),
            );
        }
        if result_count == 0 {
            list = list.child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No Utilities match. Edit the search to try again."),
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
            .p_4()
            .gap_3()
            .bg(theme.background)
            .text_color(theme.foreground)
            .text_size(px(13.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Utility Launcher"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(format!("{result_count} results")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("SEARCH UTILITIES"),
                    )
                    .child(
                        Input::new(&self.search)
                            .accessibility_id("launcher.search")
                            .w_full(),
                    ),
            )
            .child(div().h(px(1.)).bg(theme.border))
            .child(list)
            .child(
                div()
                    .pt_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("↑ ↓ navigate · Return open · Escape dismiss"),
            )
    }
}
