//! The session row actions and the open row's highlight.

use oc_state::{Action, SessionId, SidebarRow, State};

use super::{ARCHIVE, MARK_DONE, TRACK_WORK, is_open};

#[test]
fn the_row_actions_match_the_reference() {
    assert_eq!(
        [TRACK_WORK, MARK_DONE, ARCHIVE],
        ["Track as in work", "Mark done", "Archive"]
    );
}

fn row(id: &str) -> SidebarRow {
    SidebarRow {
        id: SessionId::from(id),
        title: None,
        busy: false,
    }
}

#[test]
fn only_the_open_sessions_row_is_highlighted() {
    let mut state = State::new("/tmp/project");
    let rows = [row("ses_a"), row("ses_b")];

    assert!(rows.iter().all(|row| !is_open(&state, row)));

    state.apply(Action::OpenSession(SessionId::from("ses_b")));

    let highlighted: Vec<&SessionId> = rows
        .iter()
        .filter(|row| is_open(&state, row))
        .map(|row| &row.id)
        .collect();
    assert_eq!(highlighted, vec![&SessionId::from("ses_b")]);
}
