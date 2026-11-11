//! Message queue snapshots, items and requests, against the captured fixtures.

use serde_json::Value;

use super::{
    EnqueueItem, EnqueueRequest, EnqueueResult, MessageQueue, QueueItem, QueueUpdate,
    ReorderRequest, TakenAll, TakenOne,
};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn fixture(name: &str) -> Value {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

fn assert_session_matches(item: &QueueItem, raw: &Value) {
    assert_eq!(item.id, raw["id"].as_str().expect("id"));
    assert_eq!(item.text, raw["text"].as_str().expect("text"));
    assert_eq!(item.content, raw["content"].as_str().expect("content"));
    assert_eq!(
        item.send_config.agent.as_deref(),
        raw["sendConfig"]["agent"].as_str()
    );
}

#[test]
fn the_message_queue_snapshot_decodes() {
    let captured = fixture("message-queue-list.json");
    let queue: MessageQueue = serde_json::from_value(captured.clone()).expect("queue");

    assert_eq!(
        queue.revision,
        captured["revision"].as_u64().expect("revision")
    );
    let raw = captured["sessions"].as_array().expect("sessions");
    assert_eq!(queue.sessions.len(), raw.len());
    for (session, raw) in queue.sessions.iter().zip(raw) {
        assert_eq!(
            session.session_id,
            raw["sessionId"].as_str().expect("session")
        );
        assert_eq!(
            session.directory,
            raw["directory"].as_str().expect("directory")
        );
        assert_eq!(session.sending_id, None);
        let items = raw["items"].as_array().expect("items");
        assert_eq!(session.items.len(), items.len());
        for (item, raw) in session.items.iter().zip(items) {
            assert_session_matches(item, raw);
        }
    }
}

#[test]
fn every_enqueue_result_decodes() {
    let mut names: Vec<String> = std::fs::read_dir(FIXTURES)
        .expect("fixtures/api")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("message-queue-enqueue-") && name.ends_with(".json"))
        .filter(|name| !name.ends_with(".request.json"))
        .collect();
    names.sort();
    assert!(names.len() >= 4, "{names:?}");
    for name in names {
        let captured = fixture(&name);
        let result: EnqueueResult =
            serde_json::from_value(captured.clone()).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert_eq!(
            result.revision,
            captured["revision"].as_u64().expect("revision")
        );
        assert_eq!(
            result.item_id,
            result.session.items.last().expect("an item").id
        );
        assert_eq!(result.session.session_id, captured["session"]["sessionId"]);
    }
}

#[test]
fn the_enqueue_request_serializes_to_its_saved_body() {
    let captured = fixture("message-queue-enqueue-1.request.json");
    let item: EnqueueItem = serde_json::from_value(captured["item"].clone()).expect("item");
    let request = EnqueueRequest {
        directory: captured["directory"]
            .as_str()
            .expect("directory")
            .to_owned(),
        item,
    };

    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
}

#[test]
fn the_taken_items_decode_from_their_snapshots() {
    let one = fixture("message-queue-take-one.json");
    let TakenOne {
        revision,
        session,
        item,
    } = serde_json::from_value(one.clone()).expect("take one");
    assert_eq!(revision, one["revision"].as_u64().expect("revision"));
    assert_eq!(
        session.items.len(),
        one["session"]["items"].as_array().expect("items").len()
    );
    assert_session_matches(&item, &one["item"]);

    let all = fixture("message-queue-take-all.json");
    let TakenAll {
        revision,
        session,
        items,
    } = serde_json::from_value(all.clone()).expect("take all");
    assert_eq!(revision, all["revision"].as_u64().expect("revision"));
    assert!(session.items.is_empty());
    let raw = all["items"].as_array().expect("items");
    assert_eq!(items.len(), raw.len());
    for (item, raw) in items.iter().zip(raw) {
        assert_session_matches(item, raw);
    }
}

#[test]
fn the_queue_update_and_reorder_request_match_their_fixtures() {
    let captured = fixture("message-queue-reorder.json");
    let update: QueueUpdate = serde_json::from_value(captured.clone()).expect("queue update");
    let raw = captured["session"]["items"].as_array().expect("items");
    assert_eq!(
        update.revision,
        captured["revision"].as_u64().expect("revision")
    );
    assert_eq!(update.session.items.len(), raw.len());
    for (item, raw) in update.session.items.iter().zip(raw) {
        assert_session_matches(item, raw);
    }

    let request = fixture("message-queue-reorder.request.json");
    let item_ids: Vec<String> = request["itemIds"]
        .as_array()
        .expect("item ids")
        .iter()
        .map(|value| value.as_str().expect("id").to_owned())
        .collect();
    assert_eq!(
        serde_json::to_value(ReorderRequest { item_ids }).expect("serialize"),
        request
    );
}

#[test]
fn a_scheduled_item_keeps_its_provenance_and_goal() {
    let item: QueueItem = serde_json::from_value(serde_json::json!({
        "id": "queued-1",
        "content": "text",
        "text": "text",
        "sendConfig": {},
        "scheduledTask": {"projectId": "proj_1", "taskId": "task_1"},
        "goal": {"tokenBudget": 1000},
    }))
    .expect("scheduled item");

    assert_eq!(
        item.scheduled_task
            .as_ref()
            .map(|task| task.task_id.as_str()),
        Some("task_1")
    );
    assert!(item.goal.is_some());
}
