//! The Thinking menu's labels and CSS-derived rows.

use super::{DEFAULT, NAVIGATE_HINT, SWITCH_HINT, footer_height, label_height, row_height};

#[test]
fn the_menu_names_default_and_the_levels_lowercase() {
    assert_eq!(DEFAULT, "default");
    assert_eq!(NAVIGATE_HINT, "↑↓ navigate");
    assert_eq!(SWITCH_HINT, "Ctrl + Shift + T switch");
}

#[test]
fn the_rows_are_their_css_padding_plus_one_line() {
    // `px-2 py-1.5 typography-micro` label (`MobileSheetList.tsx:11`).
    assert_eq!(label_height(), 29.);
    // `py-1 typography-meta` item (`dropdown-menu.styles.ts`, `ModelControls.tsx:2749`).
    assert_eq!(row_height(), 26.);
    // `pt-1 pb-1.5 border-t typography-micro` footer (`ModelControls.tsx:2772`).
    assert_eq!(footer_height(), 28.);
}
