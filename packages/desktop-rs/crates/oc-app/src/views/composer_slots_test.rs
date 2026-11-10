//! The footer slot order for the empty fixture.

use super::{tool_name, toolbar};
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;

#[test]
fn the_empty_state_slots_start_with_the_left_icons_and_end_with_send() {
    let state = StateScript::load(&state_path("empty"))
        .expect("fixture loads")
        .state();
    let names: Vec<String> = toolbar(&state).iter().filter_map(tool_name).collect();

    assert_eq!(names.first().map(String::as_str), Some("Add attachment"));
    assert_eq!(names.last().map(String::as_str), Some("Send message"));
    assert!(names.contains(&"Thinking: High, last used with this model".to_owned()));
}
