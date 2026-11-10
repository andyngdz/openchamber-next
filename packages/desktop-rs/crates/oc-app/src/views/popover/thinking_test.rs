//! The Thinking menu's labels.

use super::{DEFAULT, NAVIGATE_HINT, SWITCH_HINT};

#[test]
fn the_menu_names_default_and_the_levels_lowercase() {
    assert_eq!(DEFAULT, "default");
    assert_eq!(NAVIGATE_HINT, "↑↓ navigate");
    assert_eq!(SWITCH_HINT, "Ctrl + Shift + T switch");
}
