//! The popup row lists.

use super::{PopoverKind, selection_directory};

#[test]
fn a_chat_draft_reads_the_app_directory() {
    use oc_state::State;

    let state = State::new("/tmp/project");
    assert_eq!(selection_directory(&state).as_deref(), Some("/tmp/project"));
    assert_eq!(state.draft().target, oc_state::DraftTarget::Chat);
    assert_eq!(PopoverKind::Project, PopoverKind::Project);
}
