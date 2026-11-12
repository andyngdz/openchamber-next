//! The send sequence against a scripted HTTP server: dismissals, the queue take and the send.

use futures::StreamExt;
use futures::channel::mpsc;
use oc_client::form::FormInfo;
use oc_client::permission::PermissionRequest;
use oc_state::{Action, DismissKind, SessionId};

use super::execute;
use crate::live::execution::{DismissCall, SendAction, SendExecution};
use crate::live::fake_server_test::FakeServer;
use crate::live::protocol::Update;

const PROMPTED: &str = r#"{"data":{"id":"msg_1"}}"#;
const TAKEN: &str = r#"{"revision":4,"session":{"sessionId":"ses_a","items":[]},"items":[{"id":"q1","text":"first"},{"id":"q2","text":"second"}]}"#;
const ENQUEUED: &str = r#"{"revision":2,"session":{"sessionId":"ses_a","items":[]},"itemId":"q1"}"#;

fn permission_call() -> DismissCall {
    let permission: PermissionRequest =
        serde_json::from_value(serde_json::json!({"id": "per_1", "sessionID": "ses_a"}))
            .expect("permission");
    DismissCall {
        directory: "/p".to_owned(),
        session_id: "ses_a".to_owned(),
        id: "per_1".to_owned(),
        kind: DismissKind::Permission,
        permission: Some(permission),
        form: None,
    }
}

fn form_call() -> DismissCall {
    let form: FormInfo =
        serde_json::from_value(serde_json::json!({"id": "frm_1", "sessionID": "ses_a"}))
            .expect("form");
    DismissCall {
        directory: "/p".to_owned(),
        session_id: "ses_a".to_owned(),
        id: "frm_1".to_owned(),
        kind: DismissKind::Form,
        permission: None,
        form: Some(form),
    }
}

fn send(action: SendAction, dismiss: Vec<DismissCall>) -> SendExecution {
    SendExecution {
        session: SessionId::from("ses_a"),
        directory: "/p".to_owned(),
        agent: None,
        model: None,
        record_model: None,
        dismiss,
        action,
    }
}

async fn run(
    replies: Vec<(&'static str, &'static str)>,
    execution: SendExecution,
) -> (Vec<Update>, Vec<crate::live::fake_server_test::Request>) {
    let server = FakeServer::start(replies);
    let (mut sender, receiver) = mpsc::channel(8);
    execute(&server.client, execution, &mut sender)
        .await
        .expect("window open");
    drop(sender);
    let updates: Vec<Update> = receiver.collect().await;
    (updates, server.finish())
}

#[tokio::test]
async fn a_dismissing_send_rejects_and_cancels_then_queues() {
    let mut execution = send(
        SendAction::DismissThenEnqueue {
            text: Some("hello".to_owned()),
        },
        vec![permission_call(), form_call()],
    );
    execution.agent = Some("build".to_owned());
    let (updates, requests) = run(
        vec![
            ("204 No Content", ""),
            ("204 No Content", ""),
            ("200 OK", ENQUEUED),
        ],
        execution,
    )
    .await;

    let paths: Vec<&str> = requests
        .iter()
        .map(|request| request.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "/api/session/ses_a/permission/per_1/reply",
            "/api/session/ses_a/form/frm_1",
            "/api/message-queue/sessions/ses_a/items",
        ]
    );
    assert_eq!(requests[0].json()["decision"], "reject");
    let enqueued_item = &requests[2].json()["item"];
    assert_eq!(enqueued_item["text"], "hello");
    assert_eq!(enqueued_item["sendConfig"]["agent"], "build");
    let [Update::QueueUpdated(queue), Update::Queued { text }] = updates.as_slice() else {
        panic!("a queue snapshot and the queued text expected, got {updates:?}");
    };
    assert_eq!(queue.revision, 2);
    assert_eq!(text, "hello");
}

#[tokio::test]
async fn a_404_dismissal_restores_nothing() {
    let (updates, _) = run(
        vec![("404 Not Found", "{}"), ("200 OK", ENQUEUED)],
        send(
            SendAction::DismissThenEnqueue {
                text: Some("hello".to_owned()),
            },
            vec![permission_call()],
        ),
    )
    .await;

    assert!(
        updates
            .iter()
            .all(|update| !matches!(update, Update::DismissFailed { .. })),
        "{updates:?}"
    );
}

#[tokio::test]
async fn a_failed_dismissal_restores_the_request_and_still_sends() {
    let (updates, requests) = run(
        vec![("500 Internal Server Error", "{}"), ("200 OK", PROMPTED)],
        send(
            SendAction::DismissThenSend {
                take_queue: false,
                text: "hello".to_owned(),
                steer: false,
            },
            vec![form_call()],
        ),
    )
    .await;

    let [
        Update::DismissFailed { call, message },
        Update::Sent { text },
    ] = updates.as_slice()
    else {
        panic!("a restore and the prompt expected, got {updates:?}");
    };
    assert_eq!(call.id, "frm_1");
    assert!(message.contains("500"), "{message}");
    assert_eq!(text, "hello");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].path, "/api/session/ses_a/prompt");
}

#[tokio::test]
async fn a_queue_take_joins_the_texts_and_steers() {
    let (updates, requests) = run(
        vec![("200 OK", TAKEN), ("200 OK", PROMPTED)],
        send(
            SendAction::DismissThenSend {
                take_queue: true,
                text: "composer".to_owned(),
                steer: true,
            },
            Vec::new(),
        ),
    )
    .await;

    assert_eq!(requests[0].path, "/api/message-queue/sessions/ses_a/take");
    let prompt_body = requests[1].json();
    assert_eq!(prompt_body["text"], "first\n\nsecond\n\ncomposer");
    assert_eq!(prompt_body["delivery"], "steer");
    let [Update::QueueUpdated(queue), Update::Sent { text }] = updates.as_slice() else {
        panic!("the empty queue and the prompt expected, got {updates:?}");
    };
    assert!(queue.session.items.is_empty());
    assert_eq!(text, "first\n\nsecond\n\ncomposer");
}

#[tokio::test]
async fn a_failed_take_sends_nothing() {
    let (updates, requests) = run(
        vec![("500 Internal Server Error", "{}")],
        send(
            SendAction::DismissThenSend {
                take_queue: true,
                text: "composer".to_owned(),
                steer: false,
            },
            Vec::new(),
        ),
    )
    .await;

    let [Update::Apply(Action::PromptFailed(message))] = updates.as_slice() else {
        panic!("a composer error expected, got {updates:?}");
    };
    assert!(message.contains("queued"), "{message}");
    assert_eq!(requests.len(), 1);
}

#[tokio::test]
async fn a_dismissal_with_an_empty_composer_stops_the_send() {
    let (updates, requests) = run(
        vec![("204 No Content", "")],
        send(
            SendAction::DismissThenEnqueue { text: None },
            vec![permission_call()],
        ),
    )
    .await;

    assert_eq!(requests.len(), 1);
    assert!(
        matches!(updates.as_slice(), [Update::SendStopped]),
        "{updates:?}"
    );
}

#[tokio::test]
async fn an_enqueue_failure_keeps_the_draft() {
    let (updates, _) = run(
        vec![("500 Internal Server Error", "{}")],
        send(
            SendAction::Enqueue {
                text: "hello".to_owned(),
            },
            Vec::new(),
        ),
    )
    .await;

    assert!(
        matches!(updates.as_slice(), [Update::Apply(Action::PromptFailed(_))]),
        "{updates:?}"
    );
}
