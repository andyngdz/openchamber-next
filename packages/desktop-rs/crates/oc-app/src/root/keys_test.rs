//! The picker keyboard parser and the Enter modifier mapping.

use gpui_kit::Modifiers;

use super::{PopupKey, Secondary, enter_keys};

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

#[test]
fn ctrl_enter_and_meta_enter_are_separate_keys() {
    // `keyboardPolicy.ts:1-26` reads `ctrlKey` and `metaKey` separately.
    let ctrl = enter_keys(Modifiers {
        control: true,
        ..Modifiers::default()
    });
    assert!(ctrl.ctrl);
    assert!(!ctrl.meta);

    let meta = enter_keys(Modifiers {
        platform: true,
        ..Modifiers::default()
    });
    assert!(!meta.ctrl);
    assert!(meta.meta);

    let both = enter_keys(Modifiers {
        control: true,
        platform: true,
        ..Modifiers::default()
    });
    assert!(both.ctrl);
    assert!(both.meta);
}

#[test]
fn shift_and_alt_pass_through_unchanged() {
    let keys = enter_keys(Modifiers {
        shift: true,
        alt: true,
        ..Modifiers::default()
    });
    assert!(keys.shift);
    assert!(keys.alt);
    assert!(!keys.ctrl);
    assert!(!keys.meta);
}
