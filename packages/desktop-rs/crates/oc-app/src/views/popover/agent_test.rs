//! The agent menu's fixed width and accessibility label.

use super::{STAR_LABEL, WIDTH};

#[test]
fn the_agent_menu_is_280_px_wide() {
    assert_eq!(WIDTH, 280.);
}

#[test]
fn the_row_names_come_from_the_star_label() {
    assert_eq!(
        STAR_LABEL,
        "Add to favorites. With favorites, Tab cycles through them only"
    );
}
