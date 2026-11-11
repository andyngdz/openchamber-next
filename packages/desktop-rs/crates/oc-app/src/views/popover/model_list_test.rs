//! The model list's keyboard step.

use super::wrapped;

#[test]
fn a_step_wraps_at_both_ends() {
    assert_eq!(wrapped(Some(0), 1, 3), Some(1));
    assert_eq!(wrapped(Some(2), 1, 3), Some(0));
    assert_eq!(wrapped(Some(0), -1, 3), Some(2));
}

#[test]
fn a_step_without_a_selection_starts_at_the_matching_end() {
    assert_eq!(wrapped(None, 1, 3), Some(0));
    assert_eq!(wrapped(None, -1, 3), Some(2));
}

#[test]
fn an_empty_list_has_nowhere_to_step() {
    assert_eq!(wrapped(Some(0), 1, 0), None);
}
