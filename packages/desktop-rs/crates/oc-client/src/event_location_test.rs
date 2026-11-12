use crate::{Event, EventData};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/events");

fn load(name: &str) -> Vec<Event> {
    let path = format!("{FIXTURES}/{name}.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    let frames: Vec<serde_json::Value> =
        serde_json::from_str(&text).unwrap_or_else(|err| panic!("{path}: {err}"));
    frames
        .into_iter()
        .map(|frame| serde_json::from_value(frame).unwrap_or_else(|err| panic!("{name}: {err}")))
        .collect()
}

#[test]
fn envelope_location_is_kept_for_directory_events() {
    let events = load("turn-control");
    let global_form = events
        .iter()
        .find(|event| {
            matches!(&event.data, EventData::FormCreated(created) if created.form.session_id == "global")
        })
        .expect("the captured global form");
    assert!(global_form.location.directory.ends_with("/project"));
    let interrupted = events
        .iter()
        .find(|event| matches!(event.data, EventData::ExecutionInterrupted(_)))
        .expect("a captured interrupt");
    assert!(interrupted.location.directory.is_empty());
}

#[test]
fn tool_failed_keeps_the_nested_error_shape() {
    let events = load("turn-control");
    let failed = events
        .iter()
        .find_map(|event| match &event.data {
            EventData::ToolFailed(failed)
                if failed.error_message() == Some("Tool execution interrupted") =>
            {
                Some(failed)
            }
            _ => None,
        })
        .expect("the interrupted question tool");
    assert_eq!(failed.error_kind(), Some("aborted"));
    assert!(failed.error_source.is_none());
}
