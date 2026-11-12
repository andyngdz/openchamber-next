//! The design tokens the window paints with: the theme's [`Palette`] plus the depth tokens the
//! UI mockup derives from it per mode (`Tokens and depth` board, `.oc[data-mode]` CSS), and
//! their mapping onto GPUI Kit's `ThemeColor`.
//!
//! This is the one owner of the derivation. Views never recompute a glass, tile or ring
//! color; they read it from [`Tokens`], which `apply` stores as a GPUI global.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, BoxShadow, Global, Hsla, Rgba, px};

use super::shade::{BLACK, black, channels, drop_shadow, mix, white, with_alpha};
use super::{Mode, Palette, highlight};

/// Radius of the window frame, in logical pixels.
const WINDOW_RADIUS: f32 = 12.;

/// Colors that depend on the mode and the palette (`--lit`, `--ring`, `--glass`, ...).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthColors {
    /// `--lit`: the 1 px line along the top inner edge of a raised or floating surface.
    pub(crate) lit: Hsla,
    /// `--ring`: the 0.5 px inner ring of every edge.
    pub(crate) ring: Hsla,
    /// `--ring-out`: the 0.5 px dark outer ring; transparent in light themes.
    pub(crate) ring_out: Hsla,
    /// `--glass`: the background at 58% (dark) or 52% (light), over the blurred desktop.
    pub(crate) glass: Hsla,
    /// `--tile`: the foreground at 5% (dark) or 4% (light), a tone-step fill.
    pub(crate) tile: Hsla,
    /// `--tile2`: the foreground at 8% (dark) or 7% (light), a stronger tone-step fill.
    pub(crate) tile2: Hsla,
    /// `--ctl`: a raised control's fill.
    pub(crate) ctl: Hsla,
    /// `--code`: the code canvas.
    pub(crate) code: Hsla,
    /// `--scrim`: the dim layer behind a dialog.
    pub(crate) scrim: Hsla,
}

/// The theme palette and everything derived from it for one mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    pub(crate) mode: Mode,
    pub(crate) palette: Palette,
    pub(crate) depth: DepthColors,
}

impl Global for Tokens {}

impl Tokens {
    /// The tokens of a theme, derived the way the mockup CSS derives them for `mode`.
    pub(crate) fn derive(mode: Mode, palette: Palette) -> Self {
        let foreground = channels(palette.foreground);
        let background = channels(palette.background);
        let elevated = channels(palette.card);
        let depth = match mode {
            Mode::Dark => DepthColors {
                lit: white(0.08),
                ring: white(0.07),
                ring_out: black(0.6),
                glass: with_alpha(background, 0.58),
                tile: with_alpha(foreground, 0.05),
                tile2: with_alpha(foreground, 0.08),
                ctl: mix(foreground, 0.06, elevated),
                code: mix(BLACK, 0.22, background),
                scrim: black(0.45),
            },
            Mode::Light => DepthColors {
                lit: white(0.95),
                ring: black(0.08),
                ring_out: Hsla::transparent_black(),
                glass: with_alpha(background, 0.52),
                tile: with_alpha(foreground, 0.04),
                tile2: with_alpha(foreground, 0.07),
                ctl: background.into(),
                code: channels(palette.muted).into(),
                scrim: Hsla::from(Rgba {
                    r: 20. / 255.,
                    g: 20. / 255.,
                    b: 20. / 255.,
                    a: 0.18,
                }),
            },
        };
        Self {
            mode,
            palette,
            depth,
        }
    }

    /// The tokens `apply` stored for this app.
    pub fn of(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// `--sh-raised` with the edge: the shadow list of a raised control or surface.
    ///
    /// Inset shadows draw the ring and the lit line; GPUI paints them with the fill.
    pub fn raised_shadows(&self) -> Vec<BoxShadow> {
        let (near, far) = match self.mode {
            Mode::Dark => (0.4, 0.5),
            Mode::Light => (0.05, 0.14),
        };
        let mut shadows = self.edge();
        shadows.push(drop_shadow(1., 2., 0., black(near)));
        let (offset, blur, spread) = match self.mode {
            Mode::Dark => (8., 24., -8.),
            Mode::Light => (6., 18., -8.),
        };
        shadows.push(drop_shadow(offset, blur, spread, black(far)));
        shadows
    }

    /// `--sh-float` with the edge: the shadow list of a menu, popover or dialog.
    pub fn floating_shadows(&self) -> Vec<BoxShadow> {
        let (near, far) = match self.mode {
            Mode::Dark => (0.45, 0.65),
            Mode::Light => (0.08, 0.25),
        };
        let mut shadows = self.edge();
        shadows.push(drop_shadow(2., 8., 0., black(near)));
        let (offset, blur, spread) = match self.mode {
            Mode::Dark => (28., 64., -12.),
            Mode::Light => (28., 60., -14.),
        };
        shadows.push(drop_shadow(offset, blur, spread, black(far)));
        shadows
    }

    /// The edge alone: the dark outer ring, the 0.5 px inner ring and the 1 px lit top line.
    pub fn edge(&self) -> Vec<BoxShadow> {
        vec![
            BoxShadow::new(px(0.), px(0.), self.depth.ring_out).spread_radius(px(0.5)),
            BoxShadow::new(px(0.), px(0.), self.depth.ring)
                .spread_radius(px(0.5))
                .inset(),
            BoxShadow::new(px(0.), px(1.), self.depth.lit).inset(),
        ]
    }

    /// Puts the tokens on GPUI Kit's theme (the mapping of the Tokens board) and stores them.
    pub(crate) fn apply(self, cx: &mut App) {
        let palette = self.palette;
        let depth = self.depth;
        Theme::change(
            match self.mode {
                Mode::Dark => ThemeMode::Dark,
                Mode::Light => ThemeMode::Light,
            },
            None,
            cx,
        );
        let highlight_theme = highlight::for_mode(self.mode);
        Theme::update(cx, |theme| {
            theme.background = palette.background.into();
            theme.foreground = palette.foreground.into();
            theme.group_box = palette.card.into();
            theme.popover = palette.card.into();
            theme.popover_foreground = palette.foreground.into();
            theme.primary = palette.primary.into();
            theme.primary_foreground = palette.primary_foreground.into();
            theme.secondary = depth.tile;
            theme.secondary_foreground = palette.foreground.into();
            theme.muted = palette.muted.into();
            theme.muted_foreground = palette.muted_foreground.into();
            theme.accent = palette.accent.into();
            theme.accent_foreground = palette.foreground.into();
            theme.danger = palette.destructive.into();
            theme.warning = palette.status_warning.into();
            theme.success = palette.status_success.into();
            theme.info = palette.status_info.into();
            theme.border = palette.border.into();
            theme.input = palette.input.into();
            theme.ring = palette.ring.into();
            theme.list_hover = palette.hover.into();
            theme.list_active = palette.active.into();
            theme.selection = palette.selection.into();
            theme.overlay = depth.scrim;
            theme.window_border = depth.ring;
            theme.sidebar = depth.glass;
            theme.sidebar_foreground = palette.foreground.into();
            theme.sidebar_accent = palette.sidebar_accent.into();
            theme.sidebar_accent_foreground = palette.foreground.into();
            theme.sidebar_border = palette.border.into();
            theme.sidebar_primary = palette.primary.into();
            theme.sidebar_primary_foreground = palette.primary_foreground.into();
            theme.title_bar = Hsla::transparent_black();
            theme.title_bar_border = palette.border.into();
            theme.window_radius = px(WINDOW_RADIUS);
            theme.highlight_theme = highlight_theme;
        });
        cx.set_global(self);
    }
}

#[cfg(test)]
#[path = "tokens_test.rs"]
mod tokens_test;
