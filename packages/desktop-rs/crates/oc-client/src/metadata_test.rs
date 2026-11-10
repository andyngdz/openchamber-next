use serde_json::json;

use super::{MetadataUpdated, SessionMetadata, WorkState};

fn metadata(value: serde_json::Value) -> SessionMetadata {
    serde_json::from_value(value).expect("metadata decodes")
}

#[test]
fn work_state_reads_open_and_done() {
    let open = metadata(json!({"openchamber": {"work": {"state": "open", "openedBy": "auto"}}}));
    let done = metadata(json!({"openchamber": {"work": {"state": "done", "doneAt": 5}}}));

    assert_eq!(open.work_state(), Some(WorkState::Open));
    assert_eq!(done.work_state(), Some(WorkState::Done));
}

#[test]
fn missing_or_unknown_work_has_no_state() {
    let cases = [
        json!(null),
        json!({}),
        json!({"openchamber": {"turn": {"durationMs": 5}}}),
        json!({"openchamber": {"work": {"state": "paused"}}}),
        json!({"openchamber": "not an object"}),
        json!({"openchamber": {"work": {"state": 1}}}),
    ];

    for case in cases {
        assert_eq!(metadata(case.clone()).work_state(), None, "{case}");
    }
}

#[test]
fn metadata_update_decodes_with_and_without_metadata() {
    let update: MetadataUpdated = serde_json::from_value(json!({
        "sessionID": "ses_a",
        "metadata": {"openchamber": {"work": {"state": "open"}}}
    }))
    .expect("update decodes");
    let bare: MetadataUpdated =
        serde_json::from_value(json!({"sessionID": "ses_b"})).expect("bare update decodes");

    assert_eq!(update.session_id, "ses_a");
    assert_eq!(update.metadata.work_state(), Some(WorkState::Open));
    assert_eq!(bare.metadata.work_state(), None);
}
