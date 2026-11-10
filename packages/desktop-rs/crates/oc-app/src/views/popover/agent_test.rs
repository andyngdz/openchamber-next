//! The agent menu's fixed width and accessibility label.

use super::{STAR_LABEL, WIDTH, row_height};

#[test]
fn the_agent_menu_is_280_px_wide() {
    assert_eq!(WIDTH, 280.);
}

#[test]
fn a_row_is_two_px_padding_plus_the_20_px_star() {
    // `py-1` 8 + max(`typography-meta` 18, `h-5` 20) (`dropdown-menu.styles.ts`,
    // `ModelControls.tsx:139`).
    assert_eq!(row_height(), 28.);
}

#[test]
fn the_row_names_come_from_the_star_label() {
    assert_eq!(
        STAR_LABEL,
        "Add to favorites. With favorites, Tab cycles through them only"
    );
}
