//! The queue, auto-accept and routing calls against a one-shot HTTP server.

use serde_json::Value;

use crate::ClientError;
use crate::Directory;
use crate::client::client_test::{Reply, Server, fixture, head_end, ok_json, serve};
use crate::queue::EnqueueItem;
use crate::routing::PermissionMode;

const SESSION: &str = "ses_fixture";

fn json(name: &str) -> Value {
    serde_json::from_str(&fixture(name)).expect("fixture json")
}

fn body(request: &str) -> Value {
    let (_, body) = request
        .split_once(head_end().as_str())
        .expect("a body follows the head");
    serde_json::from_str(body).expect("json body")
}

#[tokio::test]
async fn the_message_queue_snapshot_is_unscoped() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/message-queue-list.json"))]);

    let queue = client.message_queue().await.expect("queue");

    let captured = json("api/message-queue-list.json");
    assert_eq!(
        queue.revision,
        captured["revision"].as_u64().expect("revision")
    );
    assert_eq!(
        queue.sessions.len(),
        captured["sessions"].as_array().expect("sessions").len()
    );
    let request = requests.recv().expect("request");
    assert!(request.starts_with("GET /api/message-queue HTTP/1.1"));
    assert!(
        !request
            .to_ascii_lowercase()
            .contains("x-opencode-directory")
    );
}

#[tokio::test]
async fn an_enqueue_posts_the_captured_body() {
    let captured = json("api/message-queue-enqueue-1.request.json");
    let item: EnqueueItem = serde_json::from_value(captured["item"].clone()).expect("item");
    let directory = Directory::new(captured["directory"].as_str().expect("directory"));
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/message-queue-enqueue-1.json"))]);

    let result = client
        .enqueue(&directory, SESSION, &item)
        .await
        .expect("enqueue");

    let response = json("api/message-queue-enqueue-1.json");
    assert_eq!(
        result.item_id,
        response["itemId"].as_str().expect("item id"),
    );
    let request = requests.recv().expect("request");
    assert!(
        request.starts_with("POST /api/message-queue/sessions/ses_fixture/items HTTP/1.1"),
        "{request}"
    );
    assert_eq!(body(&request), captured);
}

#[tokio::test]
async fn take_one_and_take_all_post_without_a_body() {
    let Server {
        client, requests, ..
    } = serve(vec![
        ok_json(fixture("api/message-queue-take-one.json")),
        ok_json(fixture("api/message-queue-take-all.json")),
    ]);

    let one = client
        .take_item(SESSION, "queued_fixture")
        .await
        .expect("take one");
    let all = client.take_all(SESSION).await.expect("take all");

    let captured_one = json("api/message-queue-take-one.json");
    assert_eq!(
        one.item.id,
        captured_one["item"]["id"].as_str().expect("id")
    );
    let captured_all = json("api/message-queue-take-all.json");
    assert_eq!(
        all.items.len(),
        captured_all["items"].as_array().expect("items").len()
    );
    let first = requests.recv().expect("first request");
    assert!(first.starts_with(
        "POST /api/message-queue/sessions/ses_fixture/items/queued_fixture/take HTTP/1.1"
    ));
    assert!(first.ends_with(head_end().as_str()), "no body: {first:?}");
    let second = requests.recv().expect("second request");
    assert!(second.starts_with("POST /api/message-queue/sessions/ses_fixture/take HTTP/1.1"));
}

#[tokio::test]
async fn a_reorder_posts_the_complete_order() {
    let captured = json("api/message-queue-reorder.request.json");
    let item_ids: Vec<String> = captured["itemIds"]
        .as_array()
        .expect("item ids")
        .iter()
        .map(|value| value.as_str().expect("id").to_owned())
        .collect();
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/message-queue-reorder.json"))]);

    let update = client.reorder(SESSION, &item_ids).await.expect("reorder");

    let response = json("api/message-queue-reorder.json");
    assert_eq!(
        update.revision,
        response["revision"].as_u64().expect("revision")
    );
    let request = requests.recv().expect("request");
    assert!(
        request.starts_with("PUT /api/message-queue/sessions/ses_fixture/order HTTP/1.1"),
        "{request}"
    );
    assert_eq!(body(&request), captured);
}

#[tokio::test]
async fn a_remove_posts_and_an_item_in_flight_is_a_status_conflict() {
    let conflict = json("api/message-queue-remove-in-flight.json");
    let Server {
        client, requests, ..
    } = serve(vec![
        ok_json(fixture("api/message-queue-remove.json")),
        Reply {
            status: "409 Conflict",
            content_type: "application/json",
            body: fixture("api/message-queue-remove-in-flight.json"),
        },
    ]);

    let update = client
        .remove_item(SESSION, "queued_fixture")
        .await
        .expect("remove");
    assert_eq!(
        update.revision,
        json("api/message-queue-remove.json")["revision"]
            .as_u64()
            .expect("revision")
    );

    match client.remove_item(SESSION, "queued_fixture").await {
        Err(ClientError::Status { status, body }) => {
            assert_eq!(status, 409);
            let error: Value = serde_json::from_str(&body).expect("error body");
            assert_eq!(error, conflict);
        }
        other => panic!("expected a status error, got {other:?}"),
    }
    assert!(requests.recv().expect("request").starts_with(
        "DELETE /api/message-queue/sessions/ses_fixture/items/queued_fixture HTTP/1.1"
    ));
}

#[tokio::test]
async fn the_auto_accept_snapshot_reads_and_the_set_posts_the_captured_mode() {
    let set_body = json("api/permission-auto-accept-set.request.json");
    let mode: PermissionMode = serde_json::from_value(set_body["mode"].clone()).expect("mode");
    let directory = Directory::new(set_body["directory"].as_str().expect("directory"));
    let Server {
        client, requests, ..
    } = serve(vec![
        ok_json(fixture("api/permission-auto-accept.json")),
        ok_json(fixture("api/permission-auto-accept-set.json")),
    ]);

    let before = client.auto_accept().await.expect("auto accept");
    let after = client
        .set_auto_accept(&directory, SESSION, mode)
        .await
        .expect("set");

    let snapshot = json("api/permission-auto-accept.json");
    assert_eq!(
        before.revision,
        snapshot["revision"].as_u64().expect("revision")
    );
    let response = json("api/permission-auto-accept-set.json");
    assert_eq!(
        after.revision,
        response["revision"].as_u64().expect("revision")
    );
    assert_eq!(
        after.modes.len(),
        response["modes"].as_object().expect("modes").len()
    );
    let first = requests.recv().expect("first request");
    assert!(first.starts_with("GET /api/permission-auto-accept HTTP/1.1"));
    let second = requests.recv().expect("second request");
    assert!(second.starts_with("PUT /api/permission-auto-accept/sessions/ses_fixture HTTP/1.1"));
    assert_eq!(body(&second), set_body);
}

#[tokio::test]
async fn routing_reads_the_captured_state() {
    let Server {
        client, requests, ..
    } = serve(vec![ok_json(fixture("api/routing.json"))]);

    let state = client.routing().await.expect("routing");

    let captured = json("api/routing.json");
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
    assert!(
        requests
            .recv()
            .expect("request")
            .starts_with("GET /api/routing HTTP/1.1")
    );
}

#[tokio::test]
async fn a_missing_routing_route_reads_as_unavailable() {
    let Server {
        client,
        requests: _requests,
        ..
    } = serve(vec![Reply {
        status: "404 Not Found",
        content_type: "text/html",
        body: "not the api".into(),
    }]);

    let state = client.routing().await.expect("routing");

    assert!(!state.available);
    assert!(!state.jev_available);
    assert!(state.held_permissions.is_empty());
}
