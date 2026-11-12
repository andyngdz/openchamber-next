//! OpenChamber's window palette, from the Electron default dark theme.
//!
//! `openchamber-dark.json` is the theme Electron applies at runtime. Its colors are resolved the
//! way `requireTheme` resolves them (`packages/ui/src/lib/theme/definition.ts`) and mapped to
//! CSS variables the way `CSSVariableGenerator` maps them
//! (`packages/ui/src/lib/theme/cssGenerator.ts`). A test keeps every token equal to the variable
//! the generator emits.

use std::sync::OnceLock;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Hsla, rgba};
use serde::Deserialize;

use crate::logging::APP;

mod highlight;
mod oklab;
mod palette;

/// `color-mix(in oklab, first 50%, second)` for two opaque colors, as `oc-title-rest`
/// mixes the foreground and the muted foreground (`packages/ui/src/index.css:217-219`).
pub(crate) use oklab::mix as mix_oklab;

/// The Electron default dark theme, copied from `packages/ui/src/lib/theme/themes/`.
pub(crate) const DARK_THEME: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/themes/openchamber-dark.json"
));

/// One theme color as `0xRRGGBBAA`, the form a CSS variable carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Color(u32);

impl TryFrom<String> for Color {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        parse_hex(&value).ok_or_else(|| format!("{value} is not a hex color"))
    }
}

impl From<Color> for Hsla {
    fn from(color: Color) -> Self {
        rgba(color.0).into()
    }
}

/// `#rrggbbaa` in sRGB, the form the parity reference records a color in.
pub(crate) fn hex(color: Hsla) -> String {
    let rgb = color.to_rgb();
    let channel = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        channel(rgb.r),
        channel(rgb.g),
        channel(rgb.b),
        channel(rgb.a)
    )
}

/// A stored hex color as a paint color.
pub(crate) fn stored(value: &str) -> Option<Hsla> {
    parse_hex(value).map(Into::into)
}

/// The same color at `alpha` times its own alpha, as a view's opacity multiplies into it.
pub(crate) fn faded(color: Hsla, alpha: f32) -> Hsla {
    let mut faded = color;
    faded.a *= alpha;
    faded
}

/// A hex color: `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`, as `color.ts` accepts.
fn parse_hex(value: &str) -> Option<Color> {
    let digits = value.trim().strip_prefix('#')?;
    let full = match digits.len() {
        3 | 4 => digits.chars().flat_map(|digit| [digit, digit]).collect(),
        6 | 8 => digits.to_owned(),
        _ => return None,
    };
    let number = u32::from_str_radix(&full, 16).ok()?;
    Some(match full.len() {
        6 => Color((number << 8) | 0xff),
        8 => Color(number),
        _ => return None,
    })
}

/// The design-system tokens the window paints with.
///
/// The status and syntax roles are the candidates the agent indicator picks from
/// (`packages/ui/src/lib/agentColors.ts`): `status_success` for Build, then the syntax roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Palette {
    pub(crate) background: Color,
    pub(crate) foreground: Color,
    pub(crate) card: Color,
    pub(crate) popover: Color,
    pub(crate) primary: Color,
    pub(crate) primary_foreground: Color,
    pub(crate) secondary: Color,
    pub(crate) muted: Color,
    pub(crate) muted_foreground: Color,
    pub(crate) accent: Color,
    pub(crate) destructive: Color,
    pub(crate) border: Color,
    pub(crate) input: Color,
    pub(crate) ring: Color,
    pub(crate) sidebar: Color,
    pub(crate) sidebar_accent: Color,
    pub(crate) status_error: Color,
    pub(crate) status_warning: Color,
    pub(crate) status_success: Color,
    pub(crate) status_info: Color,
    pub(crate) syntax_keyword: Color,
    pub(crate) syntax_type: Color,
    pub(crate) syntax_function: Color,
    pub(crate) syntax_number: Color,
    pub(crate) syntax_string: Color,
    pub(crate) syntax_operator: Color,
    pub(crate) syntax_variable: Color,
}

/// The Electron default dark theme's palette, parsed once from the embedded JSON.
pub(crate) fn palette() -> Palette {
    static PALETTE: OnceLock<Palette> = OnceLock::new();
    *PALETTE.get_or_init(|| match palette::parse(DARK_THEME) {
        Ok(palette) => palette,
        Err(error) => {
            tracing::warn!(
                target: APP,
                %error,
                "[APP] Embedded window palette did not parse"
            );
            Palette::default()
        }
    })
}

/// Switches GPUI Kit to dark mode and puts the palette on its theme colors.
pub(crate) fn apply(palette: Palette, cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    let highlight_theme = highlight::dark();
    Theme::update(cx, |theme| {
        theme.background = palette.background.into();
        theme.foreground = palette.foreground.into();
        theme.group_box = palette.card.into();
        theme.popover = palette.popover.into();
        theme.popover_foreground = palette.foreground.into();
        theme.primary = palette.primary.into();
        theme.primary_foreground = palette.primary_foreground.into();
        theme.secondary = palette.secondary.into();
        theme.secondary_foreground = palette.foreground.into();
        theme.muted = palette.muted.into();
        theme.muted_foreground = palette.muted_foreground.into();
        theme.accent = palette.accent.into();
        theme.accent_foreground = palette.foreground.into();
        theme.danger = palette.destructive.into();
        theme.border = palette.border.into();
        theme.input = palette.input.into();
        theme.ring = palette.ring.into();
        theme.sidebar = palette.sidebar.into();
        theme.sidebar_foreground = palette.foreground.into();
        theme.sidebar_accent = palette.sidebar_accent.into();
        theme.sidebar_accent_foreground = palette.foreground.into();
        theme.sidebar_border = palette.border.into();
        theme.sidebar_primary = palette.primary.into();
        theme.sidebar_primary_foreground = palette.primary_foreground.into();
        theme.title_bar = palette.background.into();
        theme.title_bar_border = palette.border.into();
        theme.highlight_theme = highlight_theme;
    });
}

/// `hashName` from `packages/ui/src/lib/agentColors.ts`.
fn hash_name(name: &str) -> u32 {
    let mut hash: i32 = 0;
    for byte in name.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(byte));
    }
    hash as u32
}

/// An agent's dot color, as `createAgentColorResolver` allocates it.
///
/// Build owns `status.success`. The remaining colors are the greedy oklab allocation the
/// resolver makes over this theme's visible syntax roles: operator, type, variable, keyword
/// (`packages/ui/src/lib/agentColors.ts:75-104`). A theme change recomputes that order; the
/// allocation is fixed for the app's one dark theme.
pub(crate) fn agent_color(name: &str, palette: &Palette) -> Color {
    if name.is_empty() || name == "build" {
        return palette.status_success;
    }
    let colors = [
        palette.syntax_operator,
        palette.syntax_type,
        palette.syntax_variable,
        palette.syntax_keyword,
    ];
    colors[(hash_name(name) as usize) % colors.len()]
}

/// The card background of the Work status panel and the composer.
pub(crate) fn card(palette: Palette) -> Hsla {
    palette.card.into()
}

#[cfg(test)]
#[path = "theme_test.rs"]
mod theme_test;
