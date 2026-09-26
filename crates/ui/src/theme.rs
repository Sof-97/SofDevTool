//! Semantic theme tokens owned by the component library.

use std::sync::RwLock;

use gpui::{rgb, Hsla};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeVariant {
    /// The fresh default identity.
    Graphite,
    /// The retained alternate dark theme.
    CatppuccinFrappe,
}

impl ThemeVariant {
    pub fn label(self) -> &'static str {
        match self {
            Self::Graphite => "Graphite",
            Self::CatppuccinFrappe => "Catppuccin Frappé",
        }
    }
}

#[derive(Clone, Copy)]
struct ThemeState {
    last_preset: ThemeVariant,
    tokens: ThemeTokens,
}

// A single application-wide palette is intentional. GPUI uses the App context
// to refresh all windows after mutations; component rendering reads this small
// immutable snapshot without passing product-owned context through each view.
static ACTIVE_THEME: RwLock<ThemeState> = RwLock::new(ThemeState {
    last_preset: ThemeVariant::Graphite,
    tokens: ThemeTokens::for_variant(ThemeVariant::Graphite),
});

/// Updates the preset backing the application-wide appearance change.
pub(crate) fn set_active_theme(variant: ThemeVariant) {
    let mut state = ACTIVE_THEME
        .write()
        .unwrap_or_else(|error| error.into_inner());
    *state = ThemeState {
        last_preset: variant,
        tokens: ThemeTokens::for_variant(variant),
    };
}

/// Returns the last selected preset. A custom palette is available through
/// [`ThemeTokens::active`] without inventing a third built-in preset.
pub fn active_theme() -> ThemeVariant {
    ACTIVE_THEME
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .last_preset
}

pub(crate) fn set_custom_tokens(tokens: ThemeTokens) {
    ACTIVE_THEME
        .write()
        .unwrap_or_else(|error| error.into_inner())
        .tokens = tokens;
}

/// Semantic colors supplied by an application. Each value is `0xRRGGBB`.
/// Keep text, focus and diagnostic colors legible against their surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemePalette {
    pub background: u32,
    pub surface: u32,
    pub surface_raised: u32,
    pub border: u32,
    pub text: u32,
    pub text_muted: u32,
    pub accent: u32,
    pub accent_text: u32,
    pub danger: u32,
    pub warning: u32,
}

impl ThemePalette {
    pub const GRAPHITE: ThemePalette = ThemePalette {
        background: 0x1b1c1f,
        surface: 0x24262b,
        surface_raised: 0x2d3037,
        border: 0x3a3e46,
        text: 0xe6e7ea,
        text_muted: 0x9aa0aa,
        accent: 0x5b8def,
        accent_text: 0x0b0d12,
        danger: 0xf0707a,
        warning: 0xe0b25e,
    };

    pub const CATPPUCCIN_FRAPPE: ThemePalette = ThemePalette {
        background: 0x303446,
        surface: 0x292c3c,
        surface_raised: 0x414559,
        border: 0x51576d,
        text: 0xc6d0f5,
        text_muted: 0x838ba7,
        accent: 0x8caaee,
        accent_text: 0x232634,
        danger: 0xe78284,
        warning: 0xe5c890,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeTokens {
    palette: ThemePalette,
}

impl ThemeTokens {
    pub const fn for_variant(variant: ThemeVariant) -> Self {
        let palette = match variant {
            ThemeVariant::Graphite => ThemePalette::GRAPHITE,
            ThemeVariant::CatppuccinFrappe => ThemePalette::CATPPUCCIN_FRAPPE,
        };
        Self { palette }
    }

    pub const fn graphite() -> Self {
        Self::for_variant(ThemeVariant::Graphite)
    }

    /// Builds a product-defined palette without exposing editor dependency types.
    pub const fn from_palette(palette: ThemePalette) -> Self {
        Self { palette }
    }

    pub const fn palette(self) -> ThemePalette {
        self.palette
    }

    /// The currently active theme's tokens.
    pub fn active() -> Self {
        ACTIVE_THEME
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .tokens
    }

    pub fn background(&self) -> Hsla {
        color(self.palette.background)
    }
    pub fn surface(&self) -> Hsla {
        color(self.palette.surface)
    }
    pub fn surface_raised(&self) -> Hsla {
        color(self.palette.surface_raised)
    }
    pub fn border(&self) -> Hsla {
        color(self.palette.border)
    }
    pub fn text(&self) -> Hsla {
        color(self.palette.text)
    }
    pub fn text_muted(&self) -> Hsla {
        color(self.palette.text_muted)
    }
    pub fn accent(&self) -> Hsla {
        color(self.palette.accent)
    }
    pub fn accent_text(&self) -> Hsla {
        color(self.palette.accent_text)
    }
    pub fn danger(&self) -> Hsla {
        color(self.palette.danger)
    }
    pub fn warning(&self) -> Hsla {
        color(self.palette.warning)
    }
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self::active()
    }
}

fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}
