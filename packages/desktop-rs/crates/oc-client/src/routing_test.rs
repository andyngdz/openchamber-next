//! The permission policy and routing state, against the captured fixtures.

use serde_json::Value;

use super::{
    AutoAcceptRequest, AutoAcceptSnapshot, HeldPermission, PermissionMode, RoutingPermissionHeld,
    RoutingState, RoutingUpdated,
};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn fixture(name: &str) -> Value {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

fn mode(value: &Value) -> PermissionMode {
    serde_json::from_value(value.clone()).expect("mode")
}

#[test]
fn the_captured_auto_accept_snapshots_decode() {
    for name in [
        "permission-auto-accept.json",
        "permission-auto-accept-set.json",
    ] {
        let captured = fixture(name);
        let snapshot: AutoAcceptSnapshot =
            serde_json::from_value(captured.clone()).unwrap_or_else(|err| panic!("{name}: {err}"));

        assert_eq!(
            snapshot.revision,
            captured["revision"].as_u64().expect("revision")
        );
        let sessions = captured["sessions"].as_object().expect("sessions");
        assert_eq!(snapshot.sessions.len(), sessions.len());
        for (session, enabled) in &snapshot.sessions {
            assert_eq!(Some(enabled), sessions[session].as_bool().as_ref());
        }
        let modes = captured["modes"].as_object().expect("modes");
        assert_eq!(snapshot.modes.len(), modes.len());
        for (session, captured_mode) in modes {
            assert_eq!(snapshot.modes[session], mode(captured_mode), "{name}");
        }
    }
}

#[test]
fn the_auto_accept_request_serializes_to_its_saved_body() {
    let captured = fixture("permission-auto-accept-set.request.json");
    let request = AutoAcceptRequest {
        mode: mode(&captured["mode"]),
        enabled: captured["enabled"].as_bool().expect("enabled"),
        directory: captured["directory"]
            .as_str()
            .expect("directory")
            .to_owned(),
    };

    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
}

#[test]
fn the_routing_state_decodes_from_the_capture() {
    let captured = fixture("routing.json");
    let state: RoutingState = serde_json::from_value(captured.clone()).expect("routing");

    assert_eq!(
        state.available,
        captured["available"].as_bool().expect("available")
    );
    assert_eq!(
        state.jev_available,
        captured["jevAvailable"].as_bool().expect("jev")
    );
    assert_eq!(
        state.held_permissions.len(),
        captured["heldPermissions"].as_array().expect("held").len()
    );
}

#[test]
fn a_routing_state_without_the_classifier_keys_still_reads_as_available() {
    let state: RoutingState = serde_json::from_value(serde_json::json!({"available": true}))
        .expect("older routing state");
    assert!(state.available);
    assert!(state.jev_available);
    assert!(state.held_permissions.is_empty());
}

#[test]
fn a_missing_routing_state_reads_as_unavailable() {
    let state = RoutingState::unavailable();
    assert!(!state.available);
    assert!(!state.jev_available);
    assert!(state.held_permissions.is_empty());
}

#[test]
fn the_held_permission_event_keeps_its_optional_fields() {
    let held: RoutingPermissionHeld = serde_json::from_value(serde_json::json!({
        "permissionId": "per_1",
        "sessionId": "ses_1",
        "directory": null,
        "score": 0.9,
        "kind": null,
    }))
    .expect("held permission");
    assert_eq!(held.permission_id, "per_1");
    assert_eq!(held.directory, None);
    assert_eq!(held.kind, None);

    let reply: HeldPermission = serde_json::from_value(serde_json::json!({
        "permissionId": "per_1",
        "score": 0.9,
        "kind": "git_history",
    }))
    .expect("held list entry");
    assert_eq!(reply.kind.as_deref(), Some("git_history"));
}

#[test]
fn the_routing_updated_event_defaults_jev_to_available() {
    let updated: RoutingUpdated =
        serde_json::from_value(serde_json::json!({"available": true})).expect("routing updated");
    assert!(updated.available);
    assert!(updated.jev_available);
}
