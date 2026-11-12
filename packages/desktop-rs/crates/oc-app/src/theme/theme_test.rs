use gpui_kit::Hsla;
use gpui_kit::rgba;

use super::{Mode, card, tokens};

#[test]
fn card_color_is_the_card_token() {
    let expected: Hsla = rgba(0x1b1918ff).into();

    assert_eq!(card(tokens(Mode::Dark).palette), expected);
}
