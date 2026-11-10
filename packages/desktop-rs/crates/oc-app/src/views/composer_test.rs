use oc_state::State;

use super::PLACEHOLDER;
use super::slots::{tool_name, toolbar};
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::views::reference_test::names as reference_names;

fn replay(name: &str) -> State {
    StateScript::load(&state_path(name))
        .expect("state fixture loads")
        .state()
}

fn composer_names(state: &State) -> Vec<String> {
    std::iter::once(PLACEHOLDER.to_owned())
        .chain(toolbar(state).iter().filter_map(tool_name))
        .collect()
}

#[test]
fn composer_controls_follow_the_reference_order() {
    for state_name in ["empty", "session", "streaming"] {
        assert_eq!(
            composer_names(&replay(state_name)),
            reference_names(state_name, "composer"),
            "{state_name}"
        );
    }
}
