//! Owner-maintained GPUI component library.
//!
//! This crate must not depend on the SofDevTool application or core crates, on
//! Utility identities, on app persistence, or on macOS service policy. The only
//! external UI dependency is `gpui-component`, used exclusively for its text
//! editing engine behind the [`TextEditor`] and [`TextField`] interfaces here.

mod components;
pub mod theme;

pub use components::*;
pub use theme::{ThemeTokens, ThemeVariant};

use gpui::{AnyView, App, AppContext as _, Entity, Render, Window};

/// Mounts a workspace view as the window root.
///
/// The dependency's root view stays the real window root, so its input registry
/// and focused-input routing keep working. The concrete type is not re-exported:
/// callers receive an opaque `Entity<impl Render>` and never name it.
pub fn mount(view: impl Into<AnyView>, window: &mut Window, cx: &mut App) -> Entity<impl Render> {
    mount_root(view, window, cx)
}

fn mount_root(
    view: impl Into<AnyView>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<gpui_component::Root> {
    cx.new(|cx| gpui_component::Root::new(view, window, cx))
}

/// Initialises the component library and its text engine.
pub fn init(cx: &mut gpui::App) {
    gpui_component::init(cx);
    components::register_key_bindings(cx);
}

/// Applies the dark appearance used by the Workbench and gallery.
pub fn set_dark_theme(window: Option<&mut gpui::Window>, cx: &mut gpui::App) {
    gpui_component::Theme::change(gpui_component::ThemeMode::Dark, window, cx);
}

/// Starts the GPUI application. Encapsulates the platform entry point so the
/// application crate does not depend on the platform crate directly.
pub fn run(on_finish_launching: impl FnOnce(&mut gpui::App) + 'static) {
    gpui_platform::application().run(on_finish_launching);
}

/// Starts the platform application after callers install process-level
/// lifecycle hooks such as macOS Dock reopen handling.
pub fn run_with_application(
    configure: impl FnOnce(&gpui::Application) + 'static,
    on_finish_launching: impl FnOnce(&mut gpui::App) + 'static,
) {
    let application = gpui_platform::application();
    configure(&application);
    application.run(on_finish_launching);
}
