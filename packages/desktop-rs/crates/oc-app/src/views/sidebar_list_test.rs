use oc_state::State;

use super::is_open;
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::views::sidebar_model::sidebar_groups;

#[test]
fn only_the_open_session_row_is_highlighted() {
    let script = StateScript::load(&state_path("session")).expect("state fixture loads");
    let mut state = State::new(script.directory);
    for action in script.actions {
        state.apply(action);
    }

    let highlighted: Vec<String> = sidebar_groups(&state)
        .iter()
        .flat_map(|group| group.rows.iter())
        .filter(|row| is_open(&state, row))
        .map(|row| row.title.clone())
        .collect();

    assert_eq!(highlighted, ["Reference: finished turn"]);
}
