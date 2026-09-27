//! Entry point for the Rust Developer Toolbox.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyWindowHandle, App, Entity, Menu, MenuItem, OsAction, TitlebarOptions, WindowBounds,
    WindowOptions,
};
use gpui_kit::component::Root;
use sofdevtool_app::clipboard::{Clipboard, GpuiClipboard};
use sofdevtool_app::history::{HistoryPolicy, HistoryRecorder, HistoryStore, SystemClock};
use sofdevtool_app::identity;
use sofdevtool_app::preferences::{HistoryPreferences, ShortcutPreferences, StartupShortcut};
use sofdevtool_app::registry::UtilityRegistry;
use sofdevtool_app::workbench::Workbench;

gpui::actions!(
    application_actions,
    [Quit, CloseWindow, ToggleFullScreen, Copy, Paste]
);

fn window_options(bounds: Option<WindowBounds>) -> WindowOptions {
    WindowOptions {
        window_bounds: bounds,
        titlebar: Some(TitlebarOptions {
            title: Some(identity::build_description().into()),
            ..Default::default()
        }),
        app_id: Some(identity::BUNDLE_IDENTIFIER.to_string()),
        ..Default::default()
    }
}

/// GPUI's macOS `active_window` reads NSApplication.mainWindow. A separate
/// Settings window or nonactivating Launcher can be key and frontmost without
/// being main, so route window menu commands by AppKit stacking order first.
fn frontmost_window(cx: &App) -> Option<AnyWindowHandle> {
    cx.window_stack()
        .and_then(|windows| windows.into_iter().next())
        .or_else(|| cx.active_window())
}

fn main() {
    let workbench: Rc<RefCell<Option<Entity<Workbench>>>> = Rc::new(RefCell::new(None));
    let visible = Rc::new(Cell::new(true));
    let native_window: Rc<RefCell<Option<AnyWindowHandle>>> = Rc::new(RefCell::new(None));
    let previous_bounds: Rc<RefCell<Option<WindowBounds>>> = Rc::new(RefCell::new(None));
    let preferences = ShortcutPreferences::application_support().ok();
    let history_preferences = HistoryPreferences::application_support().ok();
    let mut history_policy = history_preferences
        .as_ref()
        .map(HistoryPreferences::load)
        .unwrap_or_default();
    for definition in UtilityRegistry::initial().definitions() {
        history_policy.defaults.insert(
            definition.id.slug().to_owned(),
            definition.history_enabled_by_default,
        );
    }
    let persist_preferences = history_preferences.clone();
    let history = Rc::new(HistoryRecorder::with_policy(
        HistoryStore::new(
            identity::application_support_root()
                .map(|root| root.join("History"))
                .unwrap_or_else(|| {
                    std::env::temp_dir()
                        .join(identity::BUNDLE_IDENTIFIER)
                        .join("History")
                }),
        ),
        Box::new(SystemClock::new()),
        history_policy,
        persist_preferences.map(|preferences| {
            Box::new(move |policy: &HistoryPolicy| {
                let _ = preferences.save(policy);
            }) as Box<dyn Fn(&HistoryPolicy)>
        }),
    ));
    let startup = preferences
        .as_ref()
        .map(ShortcutPreferences::load_for_startup)
        .unwrap_or(StartupShortcut {
            shortcut: None,
            diagnostic: Some(
                "Launcher settings are unavailable because the Rust Application Support directory could not be resolved."
                    .into(),
            ),
        });

    let reopen_visible = visible.clone();
    let reopen_native_window = native_window.clone();
    let launch_preferences = preferences.clone();
    let application = gpui_kit::application().with_assets(gpui_kit::assets::AllAssets);
    {
        // Closing the Workbench hides its retained native window. Dock
        // reopen orders that same window front, preserving its bounds,
        // selected Utility, and child native views. Cmd-Q and the
        // application menu are the only explicit process exit paths.
        application.on_reopen(move |cx| {
            cx.activate(true);
            if !reopen_visible.get() {
                if let Some(handle) = *reopen_native_window.borrow() {
                    let restored = handle.update(cx, |_, window, _| {
                        sofdevtool_app::native_window::show(window);
                        window.activate_window();
                    });
                    if restored.is_ok() {
                        reopen_visible.set(true);
                    }
                }
            }
        });
    }
    application.run(move |cx: &mut App| {
        gpui_kit::init(cx);
        sofdevtool_app::appearance::install(cx);
        sofdevtool_app::launcher::init(cx);
        // `MenuItem::os_action` gives macOS the responder-chain selector,
        // while these bindings give its menu item the standard key
        // equivalent. The selector is therefore delivered to a focused
        // WKWebView (and remains available to GPUI controls).
        cx.bind_keys([
            gpui::KeyBinding::new("cmd-q", Quit, None),
            gpui::KeyBinding::new("cmd-w", CloseWindow, None),
            gpui::KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None),
            gpui::KeyBinding::new("cmd-c", Copy, None),
            gpui::KeyBinding::new("cmd-v", Paste, None),
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &CloseWindow, cx| {
            if let Some(target) = frontmost_window(cx) {
                // GPUI invokes global action handlers while dispatching in the
                // active window. Updating that same window here re-enters its
                // borrow and fails. Defer until the action cycle releases it.
                cx.defer(move |cx| {
                    let _ = target.update(cx, |_, window, _| {
                        sofdevtool_app::native_window::request_close(window);
                    });
                });
            }
        });
        cx.on_action(|_: &ToggleFullScreen, cx| {
            if let Some(target) = frontmost_window(cx) {
                cx.defer(move |cx| {
                    let _ = target.update(cx, |_, window, _| {
                        if window.is_resizable() {
                            window.toggle_fullscreen();
                        }
                    });
                });
            }
        });
        cx.set_menus([
            Menu::new(identity::APP_DISPLAY_NAME).items([MenuItem::action(
                format!("Quit {}", identity::APP_DISPLAY_NAME),
                Quit,
            )]),
            Menu::new("File").items([MenuItem::action("Close Window", CloseWindow)]),
            Menu::new("Edit").items([
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
            ]),
            Menu::new("View").items([MenuItem::action("Toggle Full Screen", ToggleFullScreen)]),
        ]);
        let clipboard: Rc<dyn Clipboard> = Rc::new(GpuiClipboard);
        let initial_workbench = workbench.clone();
        let initial_visible = visible.clone();
        let initial_bounds = previous_bounds.clone();
        let initial_native_window = native_window.clone();
        let preferences = launch_preferences.clone();
        let startup = startup.clone();
        let history = history.clone();
        cx.open_window(window_options(None), move |window, cx| {
            let visible = initial_visible.clone();
            let bounds = initial_bounds.clone();
            window.on_window_should_close(cx, move |window, _cx| {
                *bounds.borrow_mut() = Some(window.window_bounds());
                sofdevtool_app::native_window::hide(window);
                visible.set(false);
                // Retain this GPUI/NSWindow and its Wry child. Dock reopen
                // orders the same window front, avoiding an unsupported
                // reparent of WKWebView into a replacement NSWindow.
                false
            });
            let view = cx.new(|cx| Workbench::new(window, cx, clipboard.clone(), history.clone()));
            view.update(cx, |workbench, cx| {
                workbench.restore_shortcut_preferences(preferences, startup, cx);
            });
            *initial_native_window.borrow_mut() = Some(window.window_handle());
            *initial_workbench.borrow_mut() = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("open the workbench window");
    });
}
