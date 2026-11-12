//! The control stream's decoding and its idle limit, against the captured stream.

use std::collections::BTreeMap;
use std::time::Duration;

use futures::StreamExt;
use serde_json::Value;

use super::{ControlEvent, LeftForUser};
use crate::ClientError;
use crate::client::client_test::{Reply, Server, serve};
use crate::routing::{PermissionMode, RoutingPermissionHeld, RoutingUpdated};
use crate::sse::sse_test::trickle;

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/events");
const LF: &str = "\n";

fn frames() -> Vec<Value> {
    let path = format!("{FIXTURES}/control.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("{path}: {err}"))
}

fn event_body(frames: &[Value]) -> String {
    frames
        .iter()
        .map(|frame| format!("data: {frame}{}", LF.repeat(2)))
        .collect()
}

fn mode(value: &Value) -> PermissionMode {
    match value.as_str().expect("mode") {
        "ask" => PermissionMode::Ask,
        "safety" => PermissionMode::Safety,
        "auto" => PermissionMode::Auto,
        other => panic!("unexpected mode {other}"),
    }
}

#[tokio::test]
async fn every_captured_control_frame_decodes() {
    let captured = frames();
    let Server {
        client, requests, ..
    } = serve(vec![Reply {
        status: "200 OK",
        content_type: "text/event-stream",
        body: event_body(&captured),
    }]);

    let decoded: Vec<ControlEvent> = client
        .control_events()
        .await
        .expect("open")
        .map(|item| item.expect("control event"))
        .collect()
        .await;

    assert_eq!(decoded.len(), captured.len());
    let request = requests.recv().expect("request");
    assert!(request.starts_with("GET /api/openchamber/events HTTP/1.1"));
    assert!(request.contains("accept: text/event-stream"));
    assert!(
        !request
            .to_ascii_lowercase()
            .contains("x-opencode-directory")
    );

    let count_of = |kind: &str| {
        captured
            .iter()
            .filter(|frame| frame["type"] == kind)
            .count()
    };
    let decoded_frames = count_of("openchamber:event-stream-ready")
        + count_of("openchamber:heartbeat")
        + count_of("openchamber:message-queue.updated")
        + count_of("openchamber:permission-auto-accept.updated")
        + count_of("openchamber:permission-auto-accept.left-for-user")
        + count_of("openchamber:routing.updated")
        + count_of("openchamber:routing.permission-held");
    let seen = (
        count_of("openchamber:event-stream-ready"),
        count_of("openchamber:heartbeat"),
        count_of("openchamber:message-queue.updated"),
        count_of("openchamber:permission-auto-accept.updated"),
        count_of("openchamber:permission-auto-accept.left-for-user"),
        captured.len() - decoded_frames,
    );
    let mut counts = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    for (event, frame) in decoded.iter().zip(&captured) {
        let kind = frame["type"].as_str().expect("type");
        let properties = &frame["properties"];
        match event {
            ControlEvent::Ready => {
                assert_eq!(kind, "openchamber:event-stream-ready");
                counts.0 += 1;
            }
            ControlEvent::Heartbeat => {
                assert_eq!(kind, "openchamber:heartbeat");
                counts.1 += 1;
            }
            ControlEvent::MessageQueueUpdated(update) => {
                assert_eq!(kind, "openchamber:message-queue.updated");
                assert_eq!(
                    update.revision,
                    properties["revision"].as_u64().expect("revision")
                );
                assert_eq!(
                    update.session.session_id,
                    properties["session"]["sessionId"]
                        .as_str()
                        .expect("session")
                );
                counts.2 += 1;
            }
            ControlEvent::AutoAcceptUpdated(snapshot) => {
                assert_eq!(kind, "openchamber:permission-auto-accept.updated");
                assert_eq!(
                    snapshot.revision,
                    properties["revision"].as_u64().expect("revision")
                );
                let modes: BTreeMap<String, PermissionMode> = properties["modes"]
                    .as_object()
                    .expect("modes")
                    .iter()
                    .map(|(session, value)| (session.clone(), mode(value)))
                    .collect();
                assert_eq!(snapshot.modes, modes);
                counts.3 += 1;
            }
            ControlEvent::LeftForUser(left) => {
                assert_eq!(kind, "openchamber:permission-auto-accept.left-for-user");
                assert_eq!(
                    left.permission_id,
                    properties["permissionId"].as_str().expect("permission")
                );
                assert_eq!(
                    left.session_id,
                    properties["sessionId"].as_str().expect("session")
                );
                counts.4 += 1;
            }
            ControlEvent::RoutingUpdated(_) | ControlEvent::PermissionHeld(_) => {
                panic!("the capture has no routing frame: {kind}")
            }
            ControlEvent::Other { kind: other } => {
                assert_eq!(other, kind);
                assert!(
                    ![
                        "openchamber:event-stream-ready",
                        "openchamber:heartbeat",
                        "openchamber:message-queue.updated",
                        "openchamber:permission-auto-accept.updated",
                        "openchamber:permission-auto-accept.left-for-user",
                        "openchamber:routing.updated",
                        "openchamber:routing.permission-held",
                    ]
                    .contains(&kind)
                );
                counts.5 += 1;
            }
        }
    }
    assert_eq!(counts, seen);
    assert!(counts.0 > 0 && counts.1 > 0 && counts.2 > 0 && counts.3 > 0 && counts.4 > 0);
}

#[test]
fn an_unknown_control_event_decodes_to_other() {
    let event: ControlEvent = serde_json::from_value(
        serde_json::json!({"type": "openchamber:future.thing", "properties": {"a": 1}}),
    )
    .expect("unknown event");

    assert_eq!(
        event,
        ControlEvent::Other {
            kind: "openchamber:future.thing".to_owned()
        }
    );
}

#[test]
fn a_known_control_event_with_a_broken_payload_is_an_error() {
    let result: Result<ControlEvent, _> = serde_json::from_value(
        serde_json::json!({"type": "openchamber:message-queue.updated", "properties": {}}),
    );
    assert!(result.is_err());
}

#[test]
fn the_routing_control_events_decode() {
    let updated: ControlEvent = serde_json::from_value(serde_json::json!({
        "type": "openchamber:routing.updated",
        "properties": {"available": true, "jevAvailable": false},
    }))
    .expect("routing updated");
    assert_eq!(
        updated,
        ControlEvent::RoutingUpdated(RoutingUpdated {
            available: true,
            jev_available: false
        })
    );

    let held: ControlEvent = serde_json::from_value(serde_json::json!({
        "type": "openchamber:routing.permission-held",
        "properties": {
            "permissionId": "per_1",
            "sessionId": "ses_1",
            "directory": null,
            "score": 0.9,
            "kind": "git_history",
        },
    }))
    .expect("permission held");
    let ControlEvent::PermissionHeld(RoutingPermissionHeld {
        permission_id,
        kind,
        ..
    }) = held
    else {
        panic!("permission held expected");
    };
    assert_eq!(permission_id, "per_1");
    assert_eq!(kind.as_deref(), Some("git_history"));
}

#[test]
fn a_left_for_user_frame_keeps_a_null_directory() {
    let left: ControlEvent = serde_json::from_value(serde_json::json!({
        "type": "openchamber:permission-auto-accept.left-for-user",
        "properties": {"permissionId": "per_1", "sessionId": "ses_1", "directory": null},
    }))
    .expect("left for user");
    assert_eq!(
        left,
        ControlEvent::LeftForUser(LeftForUser {
            permission_id: "per_1".to_owned(),
            session_id: "ses_1".to_owned(),
            directory: None,
        })
    );
}

#[tokio::test]
async fn a_silent_control_stream_ends_with_an_idle_error() {
    let ready = serde_json::json!({
        "type": "openchamber:event-stream-ready",
        "properties": {},
    });
    let (client, _worker) = trickle(
        vec![format!("data: {ready}{}", LF.repeat(2))],
        Duration::ZERO,
        Duration::from_secs(5),
    );
    let client = client.with_control_idle_timeout(Duration::from_millis(300));

    let items: Vec<_> = client.control_events().await.expect("open").collect().await;

    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0].as_ref().expect("ready"), &ControlEvent::Ready);
    assert!(matches!(items[1], Err(ClientError::EventsIdle { .. })));
}

#[tokio::test]
async fn a_server_without_the_control_stream_is_not_found() {
    let Server {
        client,
        requests: _requests,
        ..
    } = serve(vec![Reply {
        status: "404 Not Found",
        content_type: "text/html",
        body: "not the api".into(),
    }]);

    assert!(matches!(
        client.control_events().await,
        Err(ClientError::NotFound { .. })
    ));
}
