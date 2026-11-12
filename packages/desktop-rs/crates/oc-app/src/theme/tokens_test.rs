use gpui_kit::{Hsla, Rgba};

use super::{Mode, Tokens};
use crate::theme::{DARK_THEME, LIGHT_THEME, palette, tokens};
use gpui_kit::TestAppContext;

fn rgba_of(color: Hsla) -> Rgba {
    Rgba::from(color)
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.002
}

#[test]
fn glass_is_the_background_at_the_mode_alpha() {
    let dark = tokens(Mode::Dark);
    let light = tokens(Mode::Light);

    assert!(close(dark.depth.glass.a, 0.58));
    assert!(close(light.depth.glass.a, 0.52));
    let background = rgba_of(Hsla::from(dark.palette.background));
    let glass = rgba_of(dark.depth.glass);
    assert!(close(glass.r, background.r) && close(glass.g, background.g));
}

#[test]
fn tiles_step_up_with_the_mode_alpha() {
    let dark = tokens(Mode::Dark).depth;
    let light = tokens(Mode::Light).depth;

    assert!(close(dark.tile.a, 0.05) && close(dark.tile2.a, 0.08));
    assert!(close(light.tile.a, 0.04) && close(light.tile2.a, 0.07));
}

#[test]
fn dark_control_fill_is_the_foreground_at_six_percent_over_elevated() {
    let tokens = tokens(Mode::Dark);
    let foreground = rgba_of(Hsla::from(tokens.palette.foreground));
    let elevated = rgba_of(Hsla::from(tokens.palette.card));
    let ctl = rgba_of(tokens.depth.ctl);

    assert!(close(ctl.r, foreground.r * 0.06 + elevated.r * 0.94));
    assert!(close(ctl.g, foreground.g * 0.06 + elevated.g * 0.94));
    assert!(close(ctl.b, foreground.b * 0.06 + elevated.b * 0.94));
    assert!(close(ctl.a, 1.));
}

#[test]
fn dark_code_canvas_is_black_at_22_percent_over_the_background() {
    let tokens = tokens(Mode::Dark);
    let background = rgba_of(Hsla::from(tokens.palette.background));
    let code = rgba_of(tokens.depth.code);

    assert!(close(code.r, background.r * 0.78));
    assert!(close(code.b, background.b * 0.78));
}

#[test]
fn light_control_fill_is_the_background_and_code_is_muted() {
    let tokens = tokens(Mode::Light);

    assert_eq!(tokens.depth.ctl, Hsla::from(tokens.palette.background));
    assert_eq!(tokens.depth.code, Hsla::from(tokens.palette.muted));
    assert_eq!(tokens.depth.ring_out.a, 0.);
}

#[test]
fn dark_edge_has_an_outer_ring_and_two_inset_lines() {
    let edge = tokens(Mode::Dark).edge();

    assert_eq!(edge.iter().filter(|shadow| shadow.inset).count(), 2);
    assert_eq!(edge.len(), 3);
}

#[test]
fn raised_and_floating_shadows_extend_the_edge() {
    let tokens = tokens(Mode::Dark);

    assert_eq!(tokens.raised_shadows().len(), tokens.edge().len() + 2);
    assert_eq!(tokens.floating_shadows().len(), tokens.edge().len() + 2);
}

#[test]
fn both_bundled_themes_derive() {
    for (mode, source) in [(Mode::Dark, DARK_THEME), (Mode::Light, LIGHT_THEME)] {
        let parsed = crate::theme::palette::parse(source).expect("bundled theme parses");

        assert_eq!(Tokens::derive(mode, parsed).mode, mode);
    }
}

#[gpui_kit::test]
fn the_palette_follows_the_applied_mode(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        tokens(Mode::Dark).apply(cx);
        assert_eq!(palette(cx), tokens(Mode::Dark).palette);
        tokens(Mode::Light).apply(cx);
        assert_eq!(palette(cx), tokens(Mode::Light).palette);
    });
}

#[test]
fn the_bundled_light_theme_matches_the_shared_source() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| dir.join("packages/web/bin/cli.js").is_file())
        .map(|root| root.join("packages/ui/src/lib/theme/themes/openchamber-light.json"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .expect("light theme source reads");

    assert_eq!(LIGHT_THEME, source);
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/shared/floating_shadows.rs"
));

#[test]
fn the_floating_shadows_are_the_ones_the_frame_check_skips() {
    let outer: Vec<(f64, f64, f64)> = tokens(Mode::Dark)
        .floating_shadows()
        .iter()
        .filter(|shadow| !shadow.inset && shadow.blur_radius > gpui_kit::px(0.))
        .map(|shadow| {
            (
                f64::from(f32::from(shadow.offset.y)),
                f64::from(f32::from(shadow.blur_radius)),
                f64::from(f32::from(shadow.spread_radius)),
            )
        })
        .collect();

    assert_eq!(outer, FLOATING_SHADOWS);
}
