//! The Thinking menu's labels.

use oc_state::State;

use super::{DEFAULT, rows};

#[test]
fn the_menu_names_default_and_the_levels_lowercase() {
    assert_eq!(DEFAULT, "default");
    let state = State::new("/tmp/project");
    let rows = rows(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "default");
    assert!(rows[0].1);
}
