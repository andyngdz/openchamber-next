use gpui_kit::WindowAppearance;

use super::mode_for;
use crate::theme::Mode;

#[test]
fn light_appearances_pick_the_light_theme() {
    assert_eq!(mode_for(WindowAppearance::Light), Mode::Light);
    assert_eq!(mode_for(WindowAppearance::VibrantLight), Mode::Light);
}

#[test]
fn dark_appearances_pick_the_dark_theme() {
    assert_eq!(mode_for(WindowAppearance::Dark), Mode::Dark);
    assert_eq!(mode_for(WindowAppearance::VibrantDark), Mode::Dark);
}
