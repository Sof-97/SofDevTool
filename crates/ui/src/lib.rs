//! sofui: reusable GPUI components and component interaction logic.
//!
//! Consumers own process startup, product policy and persistence. The complex
//! text-editing engine remains behind [`TextEditor`] and [`TextField`].

use std::sync::Arc;

mod components;
pub mod theme;

pub use components::*;
pub use theme::{active_theme, ThemePalette, ThemeTokens, ThemeVariant};

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
    components::register_segmented_control_key_bindings(cx);
    components::register_selectable_list_key_bindings(cx);
}

/// Selects one of the built-in palettes for every open window and the wrapped
/// text editor. Existing view and editor entities keep their identity, focus,
/// contents and undo history. The consumer owns persistence of the choice.
pub fn apply_theme(variant: ThemeVariant, cx: &mut App) {
    theme::set_active_theme(variant);
    sync_editor_theme(ThemeTokens::active(), cx);
    cx.refresh_windows();
}

/// Applies a caller-defined semantic palette application-wide. The caller
/// supplies legible foreground, focus and diagnostic colors and owns any
/// persistence; sofui updates all existing windows and text editors.
pub fn apply_custom_theme(tokens: ThemeTokens, cx: &mut App) {
    theme::set_custom_tokens(tokens);
    sync_editor_theme(tokens, cx);
    cx.refresh_windows();
}

fn sync_editor_theme(tokens: ThemeTokens, cx: &mut App) {
    // The dependency owns editing fundamentals; only its resolved appearance
    // is bridged here. No dependency types escape sofui's public interface.
    gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
    let theme = gpui_component::Theme::global_mut(cx);
    theme.background = tokens.background();
    theme.foreground = tokens.text();
    theme.popover = tokens.surface();
    theme.popover_foreground = tokens.text();
    theme.input = tokens.surface_raised();
    theme.border = tokens.border();
    theme.muted = tokens.surface();
    theme.muted_foreground = tokens.text_muted();
    theme.accent = tokens.accent();
    theme.accent_foreground = tokens.accent_text();
    theme.primary = tokens.accent();
    theme.primary_foreground = tokens.accent_text();
    theme.secondary = tokens.surface_raised();
    theme.secondary_foreground = tokens.text();
    theme.ring = tokens.accent();
    theme.caret = tokens.accent();
    theme.selection = tokens.accent().opacity(0.4);
    theme.danger = tokens.danger();
    theme.warning = tokens.warning();
    theme.button = tokens.surface_raised();
    theme.button_foreground = tokens.text();
    let editor_style = &mut Arc::make_mut(&mut theme.highlight_theme).style;
    editor_style.editor_background = Some(tokens.surface_raised());
    editor_style.editor_foreground = Some(tokens.text());
    editor_style.editor_gutter_background = Some(tokens.surface());
    theme.tokens = (&theme.colors).into();
    gpui_component::Theme::sync_base(cx);
}
