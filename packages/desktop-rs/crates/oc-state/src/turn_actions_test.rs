use crate::fixture_test::{api_permissions, directory, opened};
use crate::ids::SessionId;

#[test]
fn a_failed_permission_read_keeps_the_requests_on_screen() {
    let mut state = opened("ses_a");
    let requests = api_permissions("permission-request-list");
    let session = requests
        .first()
        .expect("captured request")
        .session_id
        .clone();
    state.permissions_loaded(&directory(), requests);
    assert_eq!(
        state
            .scoped_permissions(&SessionId::from(session.as_str()))
            .len(),
        1
    );
    state.permissions_failed(&directory(), "boom");
    assert!(state.requests_stale(&directory()));
    assert_eq!(
        state
            .scoped_permissions(&SessionId::from(session.as_str()))
            .len(),
        1
    );
}

#[test]
fn a_failed_auto_accept_read_keeps_the_modes() {
    let mut state = opened("ses_a");
    state.auto_accept_failed("boom");
    state.routing_failed("boom");
    assert_eq!(
        state.effective_mode("ses_a"),
        oc_client::routing::PermissionMode::Ask
    );
}

#[test]
fn a_dismissed_form_stays_closed() {
    let mut state = opened("ses_a");
    state.form_dismissed("frm_1");
    state.form_answered("frm_2");
    assert!(state.scoped_forms(&SessionId::from("ses_a")).is_empty());
}
