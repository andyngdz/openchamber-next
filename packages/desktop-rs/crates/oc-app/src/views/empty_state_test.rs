use super::{ADD_STARTER, CHIPS, CHOOSE_PROJECT, ERROR_MAX_HEIGHT, ERROR_TOP, GAP, LEFT, TOP};
use crate::views::reference_test::names as reference_names;

#[test]
fn empty_state_controls_follow_the_reference_order() {
    let mut names = vec![CHOOSE_PROJECT];
    names.extend(CHIPS.iter().map(|(name, _)| *name));
    names.push(ADD_STARTER);

    assert_eq!(names, reference_names("empty", "chatColumn"));
}

#[test]
fn the_composer_sits_where_the_reference_measured_it() {
    // Chat column top 48, then heading 48, gap 16, Choose project 28, gap 6.
    assert!((48. + TOP + 48. + 16. + 28. + GAP + 100. - 487.6_f32).abs() < 1e-3);
    assert!((280. + LEFT - 315.75_f32).abs() < f32::EPSILON);
}

#[test]
fn the_server_error_card_ends_above_the_heading() {
    const { assert!(ERROR_TOP + ERROR_MAX_HEIGHT < TOP) };
}
