//! The Oklab math `color-mix(in oklab, ...)` mixes colors in.

use gpui_kit::{Hsla, rgba};

/// One color in Oklab: perceptual lightness and the two opponent color axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Oklab {
    /// Perceptual lightness, 0 for black and 1 for white.
    pub(crate) lightness: f32,
    /// The green to red axis.
    pub(crate) a: f32,
    /// The blue to yellow axis.
    pub(crate) b: f32,
}

/// `color-mix(in oklab, first first_weight, second the rest)` for two opaque colors.
pub(crate) fn mix(first: Hsla, second: Hsla, first_weight: f32) -> Hsla {
    let first_oklab = to_oklab(first);
    let second_oklab = to_oklab(second);
    let second_weight = 1. - first_weight;
    from_oklab(Oklab {
        lightness: first_oklab.lightness * first_weight + second_oklab.lightness * second_weight,
        a: first_oklab.a * first_weight + second_oklab.a * second_weight,
        b: first_oklab.b * first_weight + second_oklab.b * second_weight,
    })
}

/// One sRGB channel to its linear value.
fn linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// One linear channel back to sRGB.
fn gamma(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        12.92 * channel
    } else {
        1.055 * channel.powf(1. / 2.4) - 0.055
    }
}

/// One color in Oklab, through the long, medium and short cone responses.
fn to_oklab(color: Hsla) -> Oklab {
    let rgb = color.to_rgb();
    let red = linear(rgb.r);
    let green = linear(rgb.g);
    let blue = linear(rgb.b);
    let long = (0.412_221_46 * red + 0.536_332_55 * green + 0.051_445_995 * blue).cbrt();
    let medium = (0.211_903_5 * red + 0.680_699_5 * green + 0.107_396_96 * blue).cbrt();
    let short = (0.088_302_46 * red + 0.281_718_85 * green + 0.629_978_7 * blue).cbrt();
    Oklab {
        lightness: 0.210_454_26 * long + 0.793_617_8 * medium - 0.004_072_047 * short,
        a: 1.977_998_5 * long - 2.428_592_2 * medium + 0.450_593_7 * short,
        b: 0.025_904_037 * long + 0.782_771_77 * medium - 0.808_675_8 * short,
    }
}

/// Back to sRGB, from an Oklab color.
fn from_oklab(color: Oklab) -> Hsla {
    let long = (color.lightness + 0.396_337_78 * color.a + 0.215_803_76 * color.b).powi(3);
    let medium = (color.lightness - 0.105_561_346 * color.a - 0.063_854_17 * color.b).powi(3);
    let short = (color.lightness - 0.089_484_18 * color.a - 1.291_485_5 * color.b).powi(3);
    let red = 4.076_741_7 * long - 3.307_711_6 * medium + 0.230_969_94 * short;
    let green = -1.268_438 * long + 2.609_757_4 * medium - 0.341_319_38 * short;
    let blue = -0.004_196_086_3 * long - 0.703_418_6 * medium + 1.707_614_7 * short;
    rgba(u32::from_be_bytes([
        (gamma(red).clamp(0., 1.) * 255.).round() as u8,
        (gamma(green).clamp(0., 1.) * 255.).round() as u8,
        (gamma(blue).clamp(0., 1.) * 255.).round() as u8,
        0xff,
    ]))
    .into()
}

#[cfg(test)]
#[path = "oklab_test.rs"]
mod oklab_test;
