use gpui_kit::rgb;

use super::{DARK, card};

#[test]
fn dark_palette_matches_the_design_system_hex_comments() {
    assert_eq!(DARK.background, 0x151313);
    assert_eq!(DARK.foreground, 0xcdccc3);
    assert_eq!(DARK.primary, 0xedb449);
    assert_eq!(DARK.border, 0x393836);
    assert_eq!(DARK.sidebar_accent, 0x282726);
}

#[test]
fn card_color_is_the_card_token() {
    let expected: gpui_kit::Hsla = rgb(0x1c1b1a).into();

    assert_eq!(card(DARK), expected);
}
