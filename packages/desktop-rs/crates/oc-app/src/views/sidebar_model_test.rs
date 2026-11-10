use oc_state::State;

use super::{GroupKind, GroupModel, sidebar_groups};
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::work::WorkTracker;

const FINISHED: &str = "Reference: finished turn";
const STREAMING: &str = "Reference: streaming turn";

fn groups_of(name: &str) -> Vec<GroupModel> {
    let script = StateScript::load(&state_path(name)).expect("state fixture loads");
    let mut state = State::new(script.directory);
    let mut work = WorkTracker::default();
    for action in script.actions {
        work.observe(&action);
        state.apply(action);
    }
    sidebar_groups(&state, &work)
}

fn titles(group: &GroupModel) -> Vec<&str> {
    group.rows.iter().map(|row| row.title.as_str()).collect()
}

#[test]
fn empty_has_one_projects_group_with_both_sessions() {
    let groups = groups_of("empty");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].kind, GroupKind::Projects);
    assert_eq!(titles(&groups[0]), [STREAMING, FINISHED]);
    assert_eq!(groups[0].rows[0].actions, ["Track as in work", "Archive"]);
}

#[test]
fn session_moves_the_finished_turn_into_work() {
    let groups = groups_of("session");

    assert_eq!(groups[0].kind, GroupKind::InWork);
    assert_eq!(titles(&groups[0]), [FINISHED]);
    assert_eq!(groups[0].rows[0].actions, ["Mark done", "Archive"]);
    assert_eq!(groups[1].kind, GroupKind::Projects);
    assert_eq!(titles(&groups[1]), [STREAMING]);
}

#[test]
fn streaming_lists_the_busy_turn_first_in_work() {
    let groups = groups_of("streaming");

    assert_eq!(titles(&groups[0]), [STREAMING, FINISHED]);
    let busy = &groups[0].rows[0];
    assert!(busy.busy);
    assert!(groups[1].rows.is_empty());
}
