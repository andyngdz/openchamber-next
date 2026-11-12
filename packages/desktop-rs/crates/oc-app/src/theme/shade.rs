//! Color arithmetic and shadow builders behind the depth tokens.

use gpui_kit::{BoxShadow, Hsla, Rgba, point, px, rgba};

use super::Color;

/// Opaque black.
pub(super) const BLACK: Rgba = Rgba {
    r: 0.,
    g: 0.,
    b: 0.,
    a: 1.,
};

/// A theme color as RGBA channels.
pub(super) fn channels(color: Color) -> Rgba {
    rgba(color.0)
}

/// White at `alpha`.
pub(super) fn white(alpha: f32) -> Hsla {
    Rgba {
        r: 1.,
        g: 1.,
        b: 1.,
        a: alpha,
    }
    .into()
}

/// Black at `alpha`.
pub(super) fn black(alpha: f32) -> Hsla {
    Rgba { a: alpha, ..BLACK }.into()
}

/// `color-mix(in srgb, color pct%, transparent)`: the color at `pct` times its own alpha.
pub(super) fn with_alpha(color: Rgba, pct: f32) -> Hsla {
    Rgba {
        a: color.a * pct,
        ..color
    }
    .into()
}

/// `color-mix(in srgb, first pct%, second)` for two opaque colors.
pub(super) fn mix(first: Rgba, pct: f32, second: Rgba) -> Hsla {
    let blend = |first: f32, second: f32| first * pct + second * (1. - pct);
    Rgba {
        r: blend(first.r, second.r),
        g: blend(first.g, second.g),
        b: blend(first.b, second.b),
        a: 1.,
    }
    .into()
}

/// The 1 px contact shadow under a raised control.
pub(super) fn contact_shadow() -> BoxShadow {
    drop_shadow(1., 2., 0., black(0.18))
}

/// An outer shadow at `offset_y` below the element.
pub(super) fn drop_shadow(offset_y: f32, blur: f32, spread: f32, color: Hsla) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(offset_y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
        inset: false,
    }
}

#[cfg(test)]
#[path = "shade_test.rs"]
mod shade_test;
