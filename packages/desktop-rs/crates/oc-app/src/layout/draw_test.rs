use gpui_kit::rgba;

use super::{IconDraw, TextDraw, collapse, image, remix};
use crate::icons::RemixIcon;

#[test]
fn collapse_matches_the_capture_whitespace_rule() {
    assert_eq!(collapse("  New   session\n"), "New session");
    assert_eq!(collapse("a\tb"), "a b");
    assert_eq!(collapse(""), "");
}

#[test]
fn an_icon_is_named_by_its_sprite_symbol_or_image() {
    let color = rgba(0x89867cff).into();
    assert_eq!(remix(RemixIcon::Menu2, color).name(), "menu-2");
    assert_eq!(
        remix(RemixIcon::Menu2, color).color().as_deref(),
        Some("#89867cff")
    );
    let logo = image("perf");
    assert_eq!(logo.name(), "img:perf");
    assert_eq!(logo.color(), None);
    assert!(matches!(logo, IconDraw::Image { .. }));
}

#[test]
fn a_text_run_carries_its_collapsed_text_and_color() {
    let run = TextDraw::new("  Context   sources ", rgba(0xdbd7ccff).into());
    assert_eq!(run.text.as_ref(), "Context sources");
    assert_eq!(run.color(), "#dbd7ccff");
}
