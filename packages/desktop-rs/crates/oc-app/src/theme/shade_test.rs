use gpui_kit::Rgba;

use super::{BLACK, black, channels, contact_shadow, drop_shadow, mix, white, with_alpha};
use crate::theme::Color;

fn rgba_of(color: gpui_kit::Hsla) -> Rgba {
    Rgba::from(color)
}

#[test]
fn channels_read_the_theme_color() {
    let color = channels(Color(0xff000080));

    assert!((color.r - 1.).abs() < f32::EPSILON);
    assert!((color.a - 128. / 255.).abs() < 0.001);
}

#[test]
fn white_and_black_carry_the_alpha() {
    assert!((rgba_of(white(0.08)).a - 0.08).abs() < 0.001);
    assert!((rgba_of(black(0.6)).a - 0.6).abs() < 0.001);
}

#[test]
fn with_alpha_scales_the_color_alpha() {
    let color = Rgba { a: 0.5, ..BLACK };

    assert!((rgba_of(with_alpha(color, 0.5)).a - 0.25).abs() < 0.001);
}

#[test]
fn mix_blends_two_opaque_colors() {
    let white = Rgba {
        r: 1.,
        g: 1.,
        b: 1.,
        a: 1.,
    };

    let mixed = rgba_of(mix(white, 0.25, BLACK));

    assert!((mixed.r - 0.25).abs() < 0.001);
    assert!((mixed.a - 1.).abs() < f32::EPSILON);
}

#[test]
fn drop_shadows_are_outer_and_contact_shadow_is_one_pixel_down() {
    let shadow = drop_shadow(4., 8., -2., black(0.5));

    assert!(!shadow.inset);
    assert_eq!(f32::from(shadow.offset.y), 4.);
    assert_eq!(f32::from(contact_shadow().offset.y), 1.);
}
