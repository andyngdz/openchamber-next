use crate::action::Action;
use crate::fixture_test::{api_sessions, directory, opened, session};
use crate::ids::SessionId;
use crate::state::State;
use crate::status::{
    ComposerStatus, ConnectionStatus, LoadStatus, Region, ServerExit, ServerFailure,
};

const FAILURE: &str = "connection refused";

#[test]
fn connection_goes_reconnecting_then_connected() {
    let mut state = opened("ses_a");
    state.apply(Action::Connection(ConnectionStatus::Connected));
    state.apply(Action::Connection(ConnectionStatus::Reconnecting {
        attempt: 1,
    }));
    state.apply(Action::Connection(ConnectionStatus::Reconnecting {
        attempt: 2,
    }));
    assert_eq!(
        state.connection(),
        &ConnectionStatus::Reconnecting { attempt: 2 }
    );
    assert!(state.open().is_some(), "messages stay on screen");

    state.apply(Action::Connection(ConnectionStatus::Connected));
    assert_eq!(state.connection(), &ConnectionStatus::Connected);
}

#[test]
fn start_failure_and_server_exit_are_states_of_their_own() {
    let mut state = State::new(directory());
    let failure = ServerFailure {
        message: "port line never arrived".to_owned(),
        stderr_tail: vec!["node: not found".to_owned()],
        log_tail: Vec::new(),
    };
    state.apply(Action::Connection(ConnectionStatus::Failed(
        failure.clone(),
    )));
    assert_eq!(state.connection(), &ConnectionStatus::Failed(failure));

    state.apply(Action::Connection(ConnectionStatus::ServerExited(
        ServerExit::Signal(9),
    )));
    assert_eq!(
        state.connection(),
        &ConnectionStatus::ServerExited(ServerExit::Signal(9))
    );
}

#[test]
fn a_sidebar_error_then_retry_goes_back_to_loading() {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(api_sessions(
        "session-list-after-turn",
    )));
    state.apply(Action::SessionsFailed(FAILURE.to_owned()));

    assert_eq!(state.region_error(Region::Sidebar), Some(FAILURE));
    assert_eq!(state.sessions().len(), 1, "a failed reload keeps the rows");

    state.apply(Action::Retry(Region::Sidebar));
    assert_eq!(state.sidebar(), &LoadStatus::Loading);
    assert_eq!(state.region_error(Region::Sidebar), None);
}

#[test]
fn a_message_error_then_retry_goes_back_to_loading() {
    let mut state = opened("ses_a");
    state.apply(Action::MessagesFailed {
        session: SessionId::from("ses_a"),
        message: FAILURE.to_owned(),
    });
    assert_eq!(state.region_error(Region::Messages), Some(FAILURE));

    state.apply(Action::Retry(Region::Messages));
    assert_eq!(
        state.open().map(|open| &open.load),
        Some(&LoadStatus::Loading)
    );
    assert_eq!(state.region_error(Region::Messages), None);
}

#[test]
fn an_answer_for_a_session_no_longer_open_is_dropped() {
    let mut state = opened("ses_a");
    state.apply(Action::OpenSession(SessionId::from("ses_b")));
    state.apply(Action::MessagesFailed {
        session: SessionId::from("ses_a"),
        message: FAILURE.to_owned(),
    });
    assert_eq!(
        state.open().map(|open| &open.load),
        Some(&LoadStatus::Loading)
    );
}

#[test]
fn the_draft_stays_after_a_failed_prompt() {
    let mut state = opened("ses_a");
    state.apply(Action::DraftChanged("fix the build".to_owned()));
    state.apply(Action::PromptSent);
    assert_eq!(state.composer().status, ComposerStatus::Sending);

    state.apply(Action::PromptFailed(FAILURE.to_owned()));
    assert_eq!(state.composer().draft, "fix the build");
    assert_eq!(state.region_error(Region::Composer), Some(FAILURE));

    state.apply(Action::Retry(Region::Composer));
    assert_eq!(state.composer().status, ComposerStatus::Idle);
    assert_eq!(state.composer().draft, "fix the build");
}

#[test]
fn an_accepted_prompt_clears_the_draft() {
    let mut state = opened("ses_a");
    state.apply(Action::DraftChanged("hello".to_owned()));
    state.apply(Action::PromptSent);
    state.apply(Action::PromptAccepted);
    assert_eq!(state.composer().draft, "");
    assert_eq!(state.composer().status, ComposerStatus::Idle);
}

#[test]
fn a_created_session_is_listed_and_open_with_no_messages() {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(Vec::new()));
    state.apply(Action::SessionCreated(session("ses_new", 1)));

    assert_eq!(state.sessions().len(), 1);
    let open = state.open().expect("opened");
    assert_eq!(open.id.as_str(), "ses_new");
    assert_eq!(open.load, LoadStatus::Ready);
    assert!(open.messages.is_empty());
}
