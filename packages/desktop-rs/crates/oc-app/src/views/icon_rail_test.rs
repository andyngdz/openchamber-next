use super::GROUPS;
use crate::views::reference_test::names as reference_names;

#[test]
fn rail_buttons_follow_the_reference_order() {
    let names: Vec<&str> = GROUPS
        .iter()
        .flat_map(|group| group.iter().map(|(name, _)| *name))
        .collect();

    for state in ["empty", "session", "streaming"] {
        assert_eq!(names, reference_names(state, "iconRail"), "{state}");
    }
}
