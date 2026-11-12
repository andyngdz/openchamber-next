use serde_json::json;

use oc_client::permission::PermissionRequest;

use crate::fixture_test::{api_forms, api_permissions, directory};
use crate::requests::Requests;
use crate::session_entry::PendingRequest;

fn request(id: &str, session: &str) -> PermissionRequest {
    serde_json::from_value(json!({"id": id, "sessionID": session})).expect("request")
}

#[test]
fn upserts_by_id_and_removes_on_reply() {
    let captured = api_permissions("permission-request-list");
    let captured_request = captured.first().expect("captured request").clone();
    let mut requests = Requests::default();
    requests.upsert_permission(captured_request.clone(), &directory());
    assert!(requests.has_permission(&captured_request.session_id, &captured_request.id));

    requests.upsert_permission(captured_request.clone(), &directory());
    assert_eq!(
        requests.permissions_for(&captured_request.session_id).len(),
        1
    );

    requests.remove_permission(&captured_request.session_id, &captured_request.id);
    assert!(
        requests
            .permissions_for(&captured_request.session_id)
            .is_empty()
    );
}

#[test]
fn keeps_each_session_list_sorted_by_id() {
    let mut requests = Requests::default();
    requests.upsert_permission(request("per_b", "ses_a"), "/repo");
    requests.upsert_permission(request("per_a", "ses_a"), "/repo");
    let ids: Vec<_> = requests
        .permissions_for("ses_a")
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(ids, ["per_a", "per_b"]);
}

#[test]
fn a_list_replaces_only_its_directory_and_a_failure_keeps_everything() {
    let mut requests = Requests::default();
    requests.upsert_permission(request("per_a", "ses_a"), "/one");
    requests.upsert_permission(request("per_b", "ses_b"), "/two");
    requests.replace_permissions("/one", Vec::new());
    assert!(requests.permissions_for("ses_a").is_empty());
    assert_eq!(requests.permissions_for("ses_b").len(), 1);

    requests.fail_permissions("/two");
    assert!(requests.is_stale("/two"));
    assert_eq!(requests.permissions_for("ses_b").len(), 1);
    requests.replace_permissions("/two", Vec::new());
    assert!(!requests.is_stale("/two"));
    assert!(requests.permissions_for("ses_b").is_empty());

    requests.fail_permissions("/one");
    requests.replace_forms("/one", Vec::new());
    assert!(requests.is_stale("/one"));
    requests.upsert_permission(request("per_event", "ses_event"), "/one");
    assert!(requests.is_stale("/one"));
}

#[test]
fn a_form_list_replaces_the_directorys_forms_and_global_forms() {
    let captured = api_forms("form-list");
    let first = captured.first().expect("the captured form");
    let mut requests = Requests::default();
    requests.replace_forms("/one", captured.clone());
    assert!(requests.has_form(&first.id));
    assert_eq!(requests.global_forms("/one").len(), 1);

    requests.replace_forms("/two", captured.clone());
    requests.replace_forms("/one", Vec::new());
    assert!(requests.global_forms("/one").is_empty());
    assert_eq!(requests.global_forms("/two").len(), 1);
    assert!(requests.has_form(&captured[0].id));
}

#[test]
fn global_forms_are_kept_per_directory() {
    let captured = api_forms("form-list");
    let global = captured
        .iter()
        .find(|form| form.session_id == "global")
        .expect("the captured global form")
        .clone();
    let mut requests = Requests::default();
    requests.upsert_form(global.clone(), "/one");
    assert_eq!(requests.global_forms("/one").len(), 1);
    assert!(requests.global_forms("/two").is_empty());
    requests.remove_form("global", &global.id);
    assert!(requests.global_forms("/one").is_empty());
}

#[test]
fn pending_marks_are_derived_for_the_sidebar() {
    let mut requests = Requests::default();
    requests.upsert_permission(request("per_a", "ses_a"), "/repo");
    let form = api_forms("form-list")
        .into_iter()
        .find(|form| form.session_id != "global")
        .expect("captured form");
    let session = form.session_id.clone();
    requests.upsert_form(form, "/repo");
    let marks = requests.pending_for(&session);
    assert!(matches!(marks.as_slice(), [PendingRequest::Form(_)]));
    assert!(matches!(
        requests.pending_for("ses_a").first(),
        Some(PendingRequest::Permission(_))
    ));
    assert_eq!(requests.pending_for("ses_a").len(), 1);
}
