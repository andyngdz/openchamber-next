use serde_json::json;

use crate::action::Action;
use crate::fixture_test::{event, opened};
use crate::ids::SessionId;

#[test]
fn a_created_form_joins_the_store_and_a_reply_removes_it() {
    let mut state = opened("ses_a");
    let form =
        json!({"form": {"id": "frm_1", "sessionID": "ses_a", "title": "Questions", "fields": []}});
    state.apply(Action::Event(event("form.created", 1, form)));
    assert!(state.open_entry().is_some_and(|entry| entry.is_waiting()));

    state.apply(Action::Event(event(
        "form.replied",
        2,
        json!({"id": "frm_1", "sessionID": "ses_a"}),
    )));
    assert!(state.scoped_forms(&SessionId::from("ses_a")).is_empty());
}

#[test]
fn a_global_form_is_filed_under_the_events_directory() {
    let mut state = opened("ses_a");
    state.apply(Action::Event(crate::fixture_test::event_in(
        "form.created",
        1,
        "/elsewhere",
        json!({"form": {"id": "frm_global", "sessionID": "global", "title": "Global", "fields": []}}),
    )));
    assert!(state.scoped_forms(&SessionId::from("ses_a")).is_empty());

    state.apply(Action::Event(crate::fixture_test::event_in(
        "form.created",
        2,
        &crate::fixture_test::directory(),
        json!({"form": {"id": "frm_global", "sessionID": "global", "title": "Global", "fields": []}}),
    )));
    let forms = state.scoped_forms(&SessionId::from("ses_a"));
    assert_eq!(forms.len(), 1);
    assert_eq!(forms[0].id, "frm_global");
}

#[test]
fn a_deleted_session_drops_its_requests() {
    let mut state = opened("ses_a");
    state.apply(Action::Event(event(
        "form.created",
        1,
        json!({"form": {"id": "frm_1", "sessionID": "ses_a", "title": "Questions", "fields": []}}),
    )));
    state.apply(Action::Event(event(
        "session.deleted",
        2,
        json!({"sessionID": "ses_a"}),
    )));
    assert!(state.scoped_forms(&SessionId::from("ses_a")).is_empty());
}
