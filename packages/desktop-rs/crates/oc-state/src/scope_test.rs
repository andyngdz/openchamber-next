use serde_json::json;

use crate::action::Action;
use crate::fixture_test::{child_session, directory, form_from, sidebar_session};
use crate::ids::SessionId;
use crate::state::State;

fn request(id: &str, session: &str) -> oc_client::permission::PermissionRequest {
    serde_json::from_value(json!({"id": id, "sessionID": session})).expect("request")
}

fn form(id: &str, session: &str) -> oc_client::form::FormInfo {
    form_from(json!({"id": id, "sessionID": session, "fields": []}))
}

fn tree() -> State {
    let mut state = State::new(directory());
    state.apply(Action::SessionsLoaded(vec![
        sidebar_session("ses_root", &directory(), 1),
        child_session("ses_child_a", "ses_root"),
        child_session("ses_child_b", "ses_root"),
        child_session("ses_grand", "ses_child_a"),
        sidebar_session("ses_sibling", &directory(), 1),
    ]));
    state
}

#[test]
fn permissions_are_root_first_then_descendants_breadth_first() {
    let mut state = tree();
    state
        .requests
        .upsert_permission(request("per_grand", "ses_grand"), &directory());
    state
        .requests
        .upsert_permission(request("per_child_b", "ses_child_b"), &directory());
    state
        .requests
        .upsert_permission(request("per_root", "ses_root"), &directory());
    state
        .requests
        .upsert_permission(request("per_sibling", "ses_sibling"), &directory());
    state
        .requests
        .upsert_permission(request("per_child_a", "ses_child_a"), &directory());

    let scoped = state.scoped_permissions(&SessionId::from("ses_root"));
    let ids: Vec<&str> = scoped
        .iter()
        .map(|entry| entry.request.id.as_str())
        .collect();
    assert_eq!(ids, ["per_root", "per_child_a", "per_child_b", "per_grand"]);
}

#[test]
fn only_a_direct_child_is_marked_from_a_subagent() {
    let mut state = tree();
    state
        .requests
        .upsert_permission(request("per_child", "ses_child_a"), &directory());
    state
        .requests
        .upsert_permission(request("per_grand", "ses_grand"), &directory());
    let scoped = state.scoped_permissions(&SessionId::from("ses_root"));
    let child = scoped
        .iter()
        .find(|entry| entry.request.session_id == "ses_child_a")
        .expect("child request");
    let grand = scoped
        .iter()
        .find(|entry| entry.request.session_id == "ses_grand")
        .expect("grandchild request");
    assert!(child.from_subagent);
    assert!(!grand.from_subagent);
}

#[test]
fn forms_are_the_tree_first_then_the_directorys_global_forms() {
    let mut state = tree();
    state
        .requests
        .upsert_form(form("frm_global", "global"), &directory());
    state
        .requests
        .upsert_form(form("frm_child", "ses_child_a"), &directory());
    state
        .requests
        .upsert_form(form("frm_root", "ses_root"), &directory());
    state
        .requests
        .upsert_form(form("frm_sibling", "ses_sibling"), &directory());

    let forms = state.scoped_forms(&SessionId::from("ses_root"));
    let ids: Vec<&str> = forms.iter().map(|form| form.id.as_str()).collect();
    assert_eq!(ids, ["frm_root", "frm_child", "frm_global"]);
}
