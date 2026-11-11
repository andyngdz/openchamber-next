//! The picker label helpers.

use oc_state::State;

use super::{selected_agent_label, selected_model, selected_model_variants, selection_directory};

#[test]
fn a_chat_draft_reads_the_app_directory() {
    let state = State::new("/tmp/project");
    assert_eq!(selection_directory(&state).as_deref(), Some("/tmp/project"));
    assert_eq!(state.draft().target, oc_state::DraftTarget::Chat);
}

#[test]
fn no_selected_model_has_no_levels_or_label() {
    let state = State::new("/tmp/project");
    assert_eq!(selected_model(&state), None);
    assert!(selected_model_variants(&state).is_empty());
    assert_eq!(selected_agent_label(&state), None);
}
