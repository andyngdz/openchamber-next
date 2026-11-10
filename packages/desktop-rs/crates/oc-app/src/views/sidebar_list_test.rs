use oc_state::{SidebarSectionModel, State};

use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;

fn replay(name: &str) -> State {
    StateScript::load(&state_path(name))
        .expect("state fixture loads")
        .state()
}

fn row_titles(state: &State) -> Vec<String> {
    state
        .sidebar_model()
        .sections
        .iter()
        .flat_map(|section| match section {
            SidebarSectionModel::Chats { rows, .. }
            | SidebarSectionModel::Pinned { rows, .. }
            | SidebarSectionModel::InWork { rows, .. } => rows.clone(),
            SidebarSectionModel::Projects { groups } => {
                groups.iter().flat_map(|group| group.rows.clone()).collect()
            }
        })
        .map(|row| row.title.unwrap_or_default())
        .collect()
}

#[test]
fn the_seed_sidebar_lists_every_seeded_session() {
    let state = replay("empty");

    assert_eq!(
        row_titles(&state),
        [
            "Reference: chat two",
            "Reference: chat one",
            "Reference: in work",
            "Reference: streaming turn",
            "Reference: finished turn",
            "Reference: second project",
        ]
    );
}

#[test]
fn the_open_session_is_listed_once() {
    let state = replay("session");
    let open = state.open().expect("a session is open").id.clone();
    let listed = state
        .sidebar_model()
        .sections
        .iter()
        .flat_map(|section| match section {
            SidebarSectionModel::Chats { rows, .. }
            | SidebarSectionModel::Pinned { rows, .. }
            | SidebarSectionModel::InWork { rows, .. } => rows.clone(),
            SidebarSectionModel::Projects { groups } => {
                groups.iter().flat_map(|group| group.rows.clone()).collect()
            }
        })
        .filter(|row| row.id == open)
        .count();

    assert_eq!(listed, 1);
}
