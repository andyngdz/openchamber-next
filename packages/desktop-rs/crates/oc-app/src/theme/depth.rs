//! The three depth levels of the mockup, as style helpers, plus the tone-step tile.
//!
//! Nothing else lifts: a control is raised, a card or the composer is a raised surface, a
//! menu, popover or dialog is a floating surface, and layout groups are tiles. Every helper
//! reads [`Tokens`], so a theme switch restyles all of them.

use gpui_kit::{Hsla, Styled, px};

use super::Tokens;

/// Radius of a control, in logical pixels.
pub(crate) const CONTROL_RADIUS: f32 = 6.;
/// Radius of a surface, in logical pixels.
pub(crate) const SURFACE_RADIUS: f32 = 10.;
/// Opacity of a floating surface's elevated fill. GPUI has no backdrop blur, so it stays near
/// opaque (`color-mix(elevated 97%, transparent)` in the mockup).
const FLOATING_FILL: f32 = 0.97;

/// Depth styling for any styled element.
pub trait DepthStyle: Styled + Sized {
    /// A raised control (button, input): the `--ctl` fill, radius 6, the edge and a 1 px
    /// contact shadow.
    fn raised_control(self, tokens: &Tokens) -> Self {
        let mut shadows = tokens.edge();
        shadows.push(super::shade::contact_shadow());
        self.bg(tokens.depth.ctl)
            .rounded(px(CONTROL_RADIUS))
            .shadow(shadows)
    }

    /// A raised surface (chat card, composer): the opaque theme background, radius 10, the
    /// edge and the raised shadow.
    fn raised_surface(self, tokens: &Tokens) -> Self {
        self.bg(Hsla::from(tokens.palette.background))
            .rounded(px(SURFACE_RADIUS))
            .shadow(tokens.raised_shadows())
    }

    /// A floating surface (menu, popover, dialog): the elevated fill at 97%, radius 10, the
    /// edge and the floating shadow.
    fn floating_surface(self, tokens: &Tokens) -> Self {
        self.bg(super::faded(tokens.palette.card.into(), FLOATING_FILL))
            .rounded(px(SURFACE_RADIUS))
            .shadow(tokens.floating_shadows())
    }

    /// A layout tile: a tone-step fill (`--tile`), radius 10 and no border. `stronger` picks
    /// `--tile2`.
    fn tile(self, tokens: &Tokens, stronger: bool) -> Self {
        let fill = if stronger {
            tokens.depth.tile2
        } else {
            tokens.depth.tile
        };
        self.bg(fill).rounded(px(SURFACE_RADIUS))
    }
}

impl<E: Styled> DepthStyle for E {}
