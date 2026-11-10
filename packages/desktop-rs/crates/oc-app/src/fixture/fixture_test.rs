use oc_state::{ConnectionStatus, SessionStatus, State};

use std::path::{Path, PathBuf};

use super::StateScript;
use crate::errors::AppError;

/// `fixtures/states/<name>.json`, found from the crate's source folder (test-only).
pub(crate) fn state_path(name: &str) -> PathBuf {
    Path::new(env!("OC_DESKTOP_FIXTURES"))
        .join("states")
        .join(format!("{name}.json"))
}

const FINISHED: &str = "Reference: finished turn";
const STREAMING: &str = "Reference: streaming turn";

fn replay(name: &str) -> State {
    StateScript::load(&state_path(name))
        .expect("state fixture loads")
        .state()
}

#[test]
fn empty_lists_the_seed_sessions_and_opens_none() {
    let state = replay("empty");

    assert_eq!(state.connection(), &ConnectionStatus::Connected);
    assert!(state.open().is_none());
    assert_eq!(state.global_sessions().len(), 6);
    assert!(state.settings().is_some());
    assert!(state.home().is_some());
    assert!(state.draft().selection.model().is_some());
}

#[test]
fn session_opens_the_finished_turn_with_its_seed_title() {
    let state = replay("session");

    let open = state.open_entry().expect("a session is open");
    assert_eq!(open.title.as_deref(), Some(FINISHED));
    assert_eq!(open.status, SessionStatus::Idle);
    assert_eq!(state.open().map(|open| open.messages.len()), Some(22));
    assert_eq!(
        state.session_selection(&open.id).and_then(|s| s.agent()),
        Some("build")
    );
}

#[test]
fn streaming_opens_a_busy_turn_and_keeps_the_finished_one_listed() {
    let state = replay("streaming");

    let open = state.open_entry().expect("a session is open");
    assert_eq!(open.title.as_deref(), Some(STREAMING));
    assert_eq!(open.status, SessionStatus::Busy);
    assert!(
        state
            .global_sessions()
            .get(&oc_state::SessionId::from("ses_reference_1"))
            .is_some()
    );
}

#[test]
fn server_error_holds_the_failure_and_no_session() {
    let state = replay("server-error");

    let ConnectionStatus::Failed(failure) = state.connection() else {
        panic!("connection is {:?}", state.connection());
    };
    assert!(!failure.stderr_tail.is_empty());
    assert!(!failure.log_tail.is_empty());
    assert!(state.global_sessions().is_empty());
    assert!(state.settings().is_none());
}

#[test]
fn a_missing_fixture_is_a_read_error() {
    let error = StateScript::load(&state_path("no-such-state")).expect_err("missing file");

    assert!(matches!(error, AppError::ReadFixture { .. }));
}

#[test]
fn an_unknown_step_is_a_decode_error() {
    let text = serde_json::json!({
        "version": 2,
        "home": { "home": "/tmp/p" },
        "settings": {},
        "steps": [{"type": "teleport"}]
    });

    let error = StateScript::parse(&text.to_string()).expect_err("unknown step");

    assert!(error.is_data());
}
