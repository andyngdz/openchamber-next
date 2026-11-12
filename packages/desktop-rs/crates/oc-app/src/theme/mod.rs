//! OpenChamber's default dark theme, from the tokens in
//! `packages/ui/src/styles/design-system.css` (Default Dark Theme, lines 164-195).
//!
//! The CSS gives each token in OKLCH with its sRGB hex in a comment; the hex is used here.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Hsla, rgb};

mod highlight;

/// The design-system tokens the window paints with, as `0xRRGGBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Palette {
    pub(crate) background: u32,
    pub(crate) foreground: u32,
    pub(crate) card: u32,
    pub(crate) popover: u32,
    pub(crate) primary: u32,
    pub(crate) primary_foreground: u32,
    pub(crate) secondary: u32,
    pub(crate) muted: u32,
    pub(crate) muted_foreground: u32,
    pub(crate) accent: u32,
    pub(crate) destructive: u32,
    pub(crate) border: u32,
    pub(crate) input: u32,
    pub(crate) ring: u32,
    pub(crate) sidebar: u32,
    pub(crate) sidebar_accent: u32,
}

/// `--background` … `--sidebar-ring` of the Default Dark Theme.
pub(crate) const DARK: Palette = Palette {
    background: 0x151313,
    foreground: 0xcdccc3,
    card: 0x1c1b1a,
    popover: 0x282726,
    primary: 0xedb449,
    primary_foreground: 0x151313,
    secondary: 0x343331,
    muted: 0x403e3c,
    muted_foreground: 0xb6b4ab,
    accent: 0x343331,
    destructive: 0xd98678,
    border: 0x393836,
    input: 0x403e3c,
    ring: 0xedb449,
    sidebar: 0x151313,
    sidebar_accent: 0x282726,
};

fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// Switches GPUI Kit to dark mode and puts the palette on its theme colors.
pub(crate) fn apply(palette: Palette, cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    let highlight_theme = highlight::dark();
    Theme::update(cx, |theme| {
        theme.background = color(palette.background);
        theme.foreground = color(palette.foreground);
        theme.group_box = color(palette.card);
        theme.popover = color(palette.popover);
        theme.popover_foreground = color(palette.foreground);
        theme.primary = color(palette.primary);
        theme.primary_foreground = color(palette.primary_foreground);
        theme.secondary = color(palette.secondary);
        theme.secondary_foreground = color(palette.foreground);
        theme.muted = color(palette.muted);
        theme.muted_foreground = color(palette.muted_foreground);
        theme.accent = color(palette.accent);
        theme.accent_foreground = color(palette.foreground);
        theme.danger = color(palette.destructive);
        theme.border = color(palette.border);
        theme.input = color(palette.input);
        theme.ring = color(palette.ring);
        theme.sidebar = color(palette.sidebar);
        theme.sidebar_foreground = color(palette.foreground);
        theme.sidebar_accent = color(palette.sidebar_accent);
        theme.sidebar_accent_foreground = color(palette.foreground);
        theme.sidebar_border = color(palette.border);
        theme.sidebar_primary = color(palette.primary);
        theme.sidebar_primary_foreground = color(palette.primary_foreground);
        theme.title_bar = color(palette.background);
        theme.title_bar_border = color(palette.border);
        theme.highlight_theme = highlight_theme;
    });
}

/// The card background of the Work status panel and the composer.
pub(crate) fn card(palette: Palette) -> Hsla {
    color(palette.card)
}

#[cfg(test)]
#[path = "theme_test.rs"]
mod theme_test;
