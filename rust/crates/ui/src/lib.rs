//! sofui: reusable GPUI components and component interaction logic.
//!
//! Consumers own process startup, product policy and persistence. The complex
//! text-editing engine remains behind [`TextEditor`] and [`TextField`].

mod components;
pub mod theme;

pub use components::*;
pub use theme::{active_theme, set_active_theme, ThemeTokens, ThemeVariant};

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
    components::register_button_key_bindings(cx);
    components::register_hold_button_key_bindings(cx);
}

/// Applies the dark appearance used by the Workbench and gallery.
pub fn set_dark_theme(window: Option<&mut gpui::Window>, cx: &mut gpui::App) {
    gpui_component::Theme::change(gpui_component::ThemeMode::Dark, window, cx);
}
