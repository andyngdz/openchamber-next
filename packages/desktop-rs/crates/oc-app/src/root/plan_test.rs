use oc_client::filesystem::Home;
use oc_state::{Selection, SidebarAction, State};

use super::{create_target, selection_parts};
use crate::live::CreateTarget;

fn state_with_home(home: Home) -> State {
    let mut state = State::new("/home/user");
    state.apply_sidebar(SidebarAction::HomeLoaded(home));
    state
}

fn home(chats_root: &str) -> Home {
    Home {
        home: "/home/user".to_owned(),
        chats_root: chats_root.to_owned(),
        canonical_chats_root: None,
        canonical_legacy_chats_root: None,
    }
}

#[test]
fn a_chat_draft_gets_a_folder_under_the_chats_root() {
    let state = state_with_home(home("/home/user/.config/openchamber/chats"));

    let Some(CreateTarget::Chat { folder }) = create_target(&state) else {
        panic!("a chat folder expected");
    };
    assert!(
        folder.starts_with("/home/user/.config/openchamber/chats/"),
        "{folder}"
    );
    assert!(folder.contains("/session-"), "{folder}");
}

#[test]
fn without_home_info_a_chat_draft_has_no_target() {
    let state = State::new("/home/user");

    assert_eq!(create_target(&state), None);
}

#[test]
fn an_empty_selection_sends_no_agent_or_model() {
    let empty = Selection::default();

    assert_eq!(selection_parts(None), (None, None));
    assert_eq!(selection_parts(Some(&empty)), (None, None));
}
