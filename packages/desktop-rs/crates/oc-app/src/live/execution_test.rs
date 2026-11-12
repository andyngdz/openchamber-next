//! A send plan as the calls the I/O task runs, and the events that take a request off screen
//! and put it back.

use oc_client::form::FormInfo;
use oc_client::permission::PermissionRequest;
use oc_client::types::Session;
use oc_state::{Action, SendKey, SessionId, State};
use serde_json::json;

use super::{from_plan, restored_event, settled_event};

fn state_with_requests() -> State {
    let mut state = State::new("/p");
    let session: Session =
        serde_json::from_value(json!({"id": "ses_a", "location": {"directory": "/p"}}))
            .expect("session");
    state.apply(Action::SessionsLoaded(vec![session]));
    state.apply(Action::OpenSession(SessionId::from("ses_a")));
    let permission: PermissionRequest =
        serde_json::from_value(json!({"id": "per_1", "sessionID": "ses_a"})).expect("permission");
    state.permissions_loaded("/p", vec![permission]);
    let form: FormInfo =
        serde_json::from_value(json!({"id": "frm_1", "sessionID": "ses_a"})).expect("form");
    state.forms_loaded("/p", vec![form]);
    state
}

#[test]
fn a_plan_resolves_its_dismissals_to_calls_with_their_records() {
    let mut state = state_with_requests();
    state.apply(Action::DraftChanged("hello".to_owned()));

    let plan = state.send_plan(SendKey::Command);
    let execution = from_plan(&plan, &state, Some("build".to_owned()), None)
        .expect("a plan with requests dismisses then queues");

    assert_eq!(execution.session, SessionId::from("ses_a"));
    assert_eq!(execution.directory, "/p");
    assert_eq!(execution.agent.as_deref(), Some("build"));
    assert_eq!(execution.dismiss.len(), 2);
    let permission = &execution.dismiss[0];
    assert_eq!(permission.directory, "/p");
    assert_eq!(permission.session_id, "ses_a");
    assert_eq!(permission.id, "per_1");
    assert!(permission.permission.is_some());
    assert!(permission.form.is_none());
    let form = &execution.dismiss[1];
    assert!(form.form.is_some());
    assert!(form.permission.is_none());
    assert_eq!(
        execution.action,
        super::SendAction::DismissThenEnqueue {
            text: Some("hello".to_owned())
        }
    );
}

#[test]
fn a_settled_event_removes_the_request_and_the_restore_puts_it_back() {
    let mut state = state_with_requests();
    let role = SessionId::from("ses_a");
    state.apply(Action::DraftChanged("hello".to_owned()));
    let plan = state.send_plan(SendKey::Command);
    let execution = from_plan(&plan, &state, None, None).expect("a plan");

    let permission = &execution.dismiss[0];
    state.apply(Action::Event(settled_event(permission).expect("an event")));
    assert!(state.scoped_permissions(&role).is_empty());

    state.apply(Action::Event(restored_event(permission).expect("an event")));
    assert_eq!(state.scoped_permissions(&role).len(), 1);

    let form = &execution.dismiss[1];
    state.apply(Action::Event(settled_event(form).expect("an event")));
    assert!(state.scoped_forms(&role).is_empty());

    state.apply(Action::Event(restored_event(form).expect("an event")));
    assert_eq!(state.scoped_forms(&role).len(), 1);
}

#[test]
fn a_plan_that_sends_nothing_has_no_execution() {
    let state = State::new("/home");

    let plan = state.send_plan(SendKey::Plain);

    assert!(from_plan(&plan, &state, None, None).is_none());
}
