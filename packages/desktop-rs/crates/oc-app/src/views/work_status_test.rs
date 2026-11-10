use oc_state::State;

use super::{PanelMode, panel_mode};
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;

fn replay(name: &str) -> State {
    StateScript::load(&state_path(name))
        .expect("state fixture loads")
        .state()
}

#[test]
fn the_card_mode_follows_the_open_turn() {
    assert_eq!(panel_mode(&replay("empty")), PanelMode::Empty);
    assert_eq!(panel_mode(&replay("session")), PanelMode::Finished);
    assert_eq!(panel_mode(&replay("streaming")), PanelMode::Running);
}
