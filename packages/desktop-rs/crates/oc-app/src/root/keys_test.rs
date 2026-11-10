//! The picker keyboard parser.

use super::{PopupKey, Secondary};

#[test]
fn arrows_escape_and_enter_parse_without_modifiers() {
    assert_eq!(
        PopupKey::parse("escape", Secondary::NotHeld),
        Some(PopupKey::Escape)
    );
    assert_eq!(
        PopupKey::parse("enter", Secondary::NotHeld),
        Some(PopupKey::Enter)
    );
    assert_eq!(
        PopupKey::parse("up", Secondary::NotHeld),
        Some(PopupKey::Up)
    );
    assert_eq!(
        PopupKey::parse("down", Secondary::NotHeld),
        Some(PopupKey::Down)
    );
}

#[test]
fn ctrl_n_and_ctrl_p_move_the_cursor() {
    assert_eq!(PopupKey::parse("n", Secondary::Held), Some(PopupKey::Next));
    assert_eq!(
        PopupKey::parse("p", Secondary::Held),
        Some(PopupKey::Previous)
    );
    assert_eq!(PopupKey::parse("n", Secondary::NotHeld), None);
}
