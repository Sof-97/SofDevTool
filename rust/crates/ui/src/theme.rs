//! Semantic theme tokens owned by the component library.

use std::sync::atomic::{AtomicU8, Ordering};

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

    fn code(self) -> u8 {
        match self {
            Self::Graphite => 0,
            Self::CatppuccinFrappe => 1,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::CatppuccinFrappe,
            _ => Self::Graphite,
        }
    }
}

/// The application's active theme. The component library is single-window and
/// UI-threaded, so one process-wide selection is sufficient and lets every
/// semantic token call site stay free of an explicit context argument.
static ACTIVE_THEME: AtomicU8 = AtomicU8::new(0);

/// Selects the process-wide theme. Callers must request a redraw.
pub fn set_active_theme(variant: ThemeVariant) {
    ACTIVE_THEME.store(variant.code(), Ordering::Release);
}

pub fn active_theme() -> ThemeVariant {
    ThemeVariant::from_code(ACTIVE_THEME.load(Ordering::Acquire))
}

#[derive(Clone, Copy, Debug)]
struct Palette {
    background: u32,
    surface: u32,
    surface_raised: u32,
    border: u32,
    text: u32,
    text_muted: u32,
    accent: u32,
    accent_text: u32,
    danger: u32,
    warning: u32,
}

impl Palette {
    const GRAPHITE: Palette = Palette {
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

    const CATPPUCCIN_FRAPPE: Palette = Palette {
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

#[derive(Clone, Copy, Debug)]
pub struct ThemeTokens {
    palette: Palette,
}

impl ThemeTokens {
    pub const fn for_variant(variant: ThemeVariant) -> Self {
        let palette = match variant {
            ThemeVariant::Graphite => Palette::GRAPHITE,
            ThemeVariant::CatppuccinFrappe => Palette::CATPPUCCIN_FRAPPE,
        };
        Self { palette }
    }

    pub const fn graphite() -> Self {
        Self::for_variant(ThemeVariant::Graphite)
    }

    /// The currently active theme's tokens.
    pub fn active() -> Self {
        Self::for_variant(active_theme())
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
