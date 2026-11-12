//! Pending permission requests and their replies, against the captured fixtures.

use serde_json::Value;

use super::{PermissionReply, PermissionReplyRequest, PermissionRequest, PermissionSourceKind};
use crate::types::Located;

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn fixture(name: &str) -> Value {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

#[test]
fn the_pending_permission_list_decodes_into_the_captured_requests() {
    let captured = fixture("permission-request-list.json");
    let located: Located<Vec<PermissionRequest>> =
        serde_json::from_value(captured.clone()).expect("permission list");
    let raw = captured["data"].as_array().expect("data");

    assert_eq!(located.data.len(), raw.len());
    assert_eq!(
        located.location.directory,
        captured["location"]["directory"]
    );
    for (request, raw) in located.data.iter().zip(raw) {
        assert_eq!(request.id, raw["id"].as_str().expect("id"));
        assert_eq!(
            request.session_id,
            raw["sessionID"].as_str().expect("session")
        );
        assert_eq!(request.action, raw["action"].as_str().expect("action"));
        assert_eq!(
            request.resources,
            raw["resources"]
                .as_array()
                .expect("resources")
                .iter()
                .map(|value| value.as_str().expect("resource").to_owned())
                .collect::<Vec<_>>()
        );
        let source = request.source.as_ref().expect("source");
        assert_eq!(source.kind, PermissionSourceKind::Tool);
        let raw_source = &raw["source"];
        assert_eq!(
            source.message_id,
            raw_source["messageID"].as_str().expect("message")
        );
        assert_eq!(source.id, raw_source["id"].as_str().expect("call"));
    }
}

#[test]
fn a_request_naming_only_its_ids_still_decodes() {
    let request: PermissionRequest =
        serde_json::from_value(serde_json::json!({"id": "per_x", "sessionID": "ses_x"}))
            .expect("bare request");

    assert_eq!(request.action, "");
    assert!(request.resources.is_empty());
    assert!(request.save.is_empty());
    assert!(request.metadata.is_empty());
    assert_eq!(request.source, None);
}

#[test]
fn every_captured_decision_serializes_to_its_saved_body() {
    let names = [
        "permission-reply-once",
        "permission-reply-reject",
        "permission-reply-always",
    ];
    for name in names {
        let captured = fixture(&format!("{name}.request.json"));
        let request: PermissionReplyRequest =
            serde_json::from_value(captured.clone()).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert!(matches!(
            request.decision,
            PermissionReply::Once | PermissionReply::Reject | PermissionReply::Always
        ));
        assert_eq!(
            serde_json::to_value(&request).expect("serialize"),
            captured,
            "{name}"
        );
    }
}
