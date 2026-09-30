//! Application appearance: System, Light and Dark over GPUI Kit's official
//! Catppuccin presets.
//!
//! GPUI Kit's [`ThemeMode`] only distinguishes light and dark. The owner-facing
//! choice adds `System`, which resolves to the current macOS appearance and is
//! re-resolved while the application runs. The bundled
//! [`catppuccin.json`](../assets/themes/catppuccin.json) derives from GPUI Kit's
//! theme set, with syntax colors corrected to Catppuccin's official palette
//! and darker Latte line numbers. Light selects Latte and Dark selects Frappe.
//! This module owns the mode and applies those presets to the kit.

use gpui::{App, Window};
use gpui_kit::component::{Theme, ThemeConfig, ThemeMode, ThemeRegistry};

/// The GPUI Kit Catppuccin presets with local syntax corrections, bundled offline.
const CATPPUCCIN_THEMES: &str = include_str!("../assets/themes/catppuccin.json");

/// The official Light preset used for Light appearance.
pub const LIGHT_THEME_NAME: &str = "Catppuccin Latte";
/// The official Dark preset used for Dark appearance.
pub const DARK_THEME_NAME: &str = "Catppuccin Frappe";

/// The owner's appearance preference, stored in workspace preferences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppearanceMode {
    /// Follow the macOS appearance and react to changes while running.
    #[default]
    System,
    /// Catppuccin Latte.
    Light,
    /// Catppuccin Frappe.
    Dark,
}

impl AppearanceMode {
    /// The value persisted in workspace preferences.
    pub fn stored_value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Resolves a stored preference into an appearance.
    ///
    /// `system`, `light` and `dark` are the current values. The retired
    /// Graphite and Catppuccin theme names both map to Dark so a previously
    /// dark appearance is preserved; a missing or unrecognized value starts in
    /// System. Because this is a pure mapping, re-reading a legacy value is
    /// repeatable and a subsequently saved mode is never overwritten.
    pub fn from_stored(value: &str) -> Self {
        match value {
            "system" => Self::System,
            "light" => Self::Light,
            "dark" | "graphite" | "catppuccin" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

struct AppearanceInstalled;

impl gpui::Global for AppearanceInstalled {}

fn ensure_themes(cx: &mut App) {
    if cx.has_global::<AppearanceInstalled>() {
        return;
    }
    ThemeRegistry::global_mut(cx)
        .load_themes_from_str(CATPPUCCIN_THEMES)
        .expect("the bundled Catppuccin presets parse");
    cx.set_global(AppearanceInstalled);
}

/// Installs the bundled themes without changing the active appearance.
///
/// Call once at startup before applying a mode. Idempotent.
pub fn install(cx: &mut App) {
    ensure_themes(cx);
}

/// Selects the Light/Dark presets and applies the effective appearance to every
/// window. Existing views and editors keep their identity, focus, contents and
/// undo history; only colors change.
pub fn apply(mode: AppearanceMode, window: Option<&mut Window>, cx: &mut App) {
    ensure_themes(cx);
    let latte = preset(cx, LIGHT_THEME_NAME);
    let frappe = preset(cx, DARK_THEME_NAME);
    {
        let theme = Theme::global_mut(cx);
        if let Some(latte) = latte {
            theme.light_theme = latte;
        }
        if let Some(frappe) = frappe {
            theme.dark_theme = frappe;
        }
    }
    let effective = match mode {
        AppearanceMode::System => ThemeMode::from(cx.window_appearance()),
        AppearanceMode::Light => ThemeMode::Light,
        AppearanceMode::Dark => ThemeMode::Dark,
    };
    Theme::change(effective, window, cx);
    cx.refresh_windows();
}

/// Re-resolves System mode against the current macOS appearance.
///
/// Call when the window's appearance changes while the stored mode is System.
pub fn apply_system(window: &mut Window, cx: &mut App) {
    apply(AppearanceMode::System, Some(window), cx);
}

/// Whether the resolved appearance currently in effect is dark.
pub fn effective_is_dark(cx: &App) -> bool {
    Theme::global(cx).is_dark()
}

fn preset(cx: &App, name: &str) -> Option<std::rc::Rc<ThemeConfig>> {
    ThemeRegistry::global(cx).themes().get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_theme_names_map_to_dark() {
        assert_eq!(
            AppearanceMode::from_stored("graphite"),
            AppearanceMode::Dark
        );
        assert_eq!(
            AppearanceMode::from_stored("catppuccin"),
            AppearanceMode::Dark
        );
    }

    #[test]
    fn missing_or_unknown_values_start_in_system() {
        assert_eq!(AppearanceMode::from_stored(""), AppearanceMode::System);
        assert_eq!(AppearanceMode::from_stored("mocha"), AppearanceMode::System);
    }

    #[test]
    fn current_values_round_trip() {
        for mode in [
            AppearanceMode::System,
            AppearanceMode::Light,
            AppearanceMode::Dark,
        ] {
            assert_eq!(AppearanceMode::from_stored(mode.stored_value()), mode);
        }
    }
}
