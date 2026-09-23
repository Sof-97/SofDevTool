//! Entry point for the Rust Developer Toolbox.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyWindowHandle, App, Entity, Menu, MenuItem, OsAction, TitlebarOptions, WindowBounds,
    WindowOptions,
};
use sofdevtool_app::clipboard::{Clipboard, GpuiClipboard};
use sofdevtool_app::history::{HistoryPolicy, HistoryRecorder, HistoryStore, SystemClock};
use sofdevtool_app::identity;
use sofdevtool_app::preferences::{HistoryPreferences, ShortcutPreferences, StartupShortcut};
use sofdevtool_app::registry::{OpenUtility, UtilityId, UtilityRegistry};
use sofdevtool_app::workbench::Workbench;
use sofdevtool_ui::{apply_theme, init, mount, ThemeVariant};

gpui::actions!(application_actions, [Quit, Copy, Paste]);

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

fn main() {
    // A deterministic native Ticket 02 check: launches the ordinary
    // application composition with Text Diff selected, without requiring the
    // Ticket 03 launcher panel or a registered global shortcut.
    let text_diff_proof = std::env::args().any(|argument| argument == "--text-diff-proof");
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
    let application = gpui_platform::application();
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
        init(cx);
        sofdevtool_app::launcher::init(cx);
        apply_theme(ThemeVariant::Graphite, cx);
        // `MenuItem::os_action` gives macOS the responder-chain selector,
        // while these bindings give its menu item the standard key
        // equivalent. The selector is therefore delivered to a focused
        // WKWebView (and remains available to GPUI controls).
        cx.bind_keys([
            gpui::KeyBinding::new("cmd-q", Quit, None),
            gpui::KeyBinding::new("cmd-c", Copy, None),
            gpui::KeyBinding::new("cmd-v", Paste, None),
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.set_menus([
            Menu::new(identity::APP_DISPLAY_NAME).items([MenuItem::action(
                format!("Quit {}", identity::APP_DISPLAY_NAME),
                Quit,
            )]),
            Menu::new("Edit").items([
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
            ]),
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
                if text_diff_proof {
                    workbench.open(OpenUtility(UtilityId::TextDiff), cx);
                }
            });
            *initial_native_window.borrow_mut() = Some(window.window_handle());
            *initial_workbench.borrow_mut() = Some(view.clone());
            mount(view, window, cx)
        })
        .expect("open the workbench window");
    });
}
