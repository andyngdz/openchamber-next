//! Mixing two colors in Oklab.

use gpui_kit::{Hsla, rgba};

use super::mix;

#[test]
fn a_color_mixed_with_itself_is_unchanged() {
    let color: Hsla = rgba(0x3366_99ff).into();

    assert_eq!(crate::theme::hex(mix(color, color, 0.5)), "#336699ff");
}

#[test]
fn black_and_white_mix_to_a_mid_gray() {
    let black: Hsla = rgba(0x0000_00ff).into();
    let white: Hsla = rgba(0xffff_ffff).into();
    let gray = mix(black, white, 0.5);
    let rgb = gray.to_rgb();

    // Oklab's perceptual midpoint is lighter than sRGB's: a mid gray near 0x63.
    assert!((rgb.r - rgb.g).abs() < 0.01);
    assert!((rgb.g - rgb.b).abs() < 0.01);
    assert!((rgb.r - 0.389).abs() < 0.01, "gray was {rgb:?}");
}
