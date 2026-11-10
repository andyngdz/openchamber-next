use super::{Event, EventData};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/events");

const FILES: [&str; 6] = [
    "text-reply-stream-300cps",
    "agent-20tools-300cps",
    "interrupted-agent-turn",
    "permission-asked-agent-turn",
    "two-sessions-stream-300cps",
    "session-lifecycle",
];

fn load(name: &str) -> Vec<Event> {
    let path = format!("{FIXTURES}/{name}.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    let frames: Vec<serde_json::Value> =
        serde_json::from_str(&text).unwrap_or_else(|err| panic!("{path}: {err}"));
    frames
        .into_iter()
        .enumerate()
        .map(|(index, frame)| {
            serde_json::from_value(frame)
                .unwrap_or_else(|err| panic!("{name} event {index}: {err}"))
        })
        .collect()
}

fn count(events: &[Event], matches: impl Fn(&EventData) -> bool) -> usize {
    events.iter().filter(|e| matches(&e.data)).count()
}

#[test]
fn every_captured_event_decodes() {
    for name in FILES {
        let events = load(name);
        assert!(
            matches!(
                events.first().map(|e| &e.data),
                Some(EventData::ServerConnected)
            ),
            "{name}"
        );
    }
}

#[test]
fn text_reply_has_the_streaming_sequence() {
    let events = load("text-reply-stream-300cps");
    let position = |wanted: fn(&EventData) -> bool| events.iter().position(|e| wanted(&e.data));
    let started = position(|d| matches!(d, EventData::TextStarted(_))).expect("text.started");
    let ended = position(|d| matches!(d, EventData::TextEnded(_))).expect("text.ended");
    let deltas: Vec<(usize, &str)> = events
        .iter()
        .enumerate()
        .filter_map(|(index, e)| match &e.data {
            EventData::TextDelta(d) => Some((index, d.delta.as_str())),
            _ => None,
        })
        .collect();
    assert!(!deltas.is_empty());
    assert!(
        deltas
            .iter()
            .all(|(index, _)| started < *index && *index < ended)
    );
    let EventData::TextEnded(full) = &events[ended].data else {
        panic!("text.ended expected");
    };
    let joined: String = deltas.iter().map(|(_, delta)| *delta).collect();
    assert_eq!(joined, full.text);
    let enqueued = position(|d| matches!(d, EventData::InboxEnqueued(_))).expect("enqueued");
    let busy = position(|d| matches!(d, EventData::ExecutionStarted(_))).expect("started");
    let idle = position(|d| matches!(d, EventData::ExecutionSucceeded(_))).expect("succeeded");
    assert!(enqueued < busy && busy < started && ended < idle);
}

#[test]
fn notices_the_spike_does_not_handle_decode_to_raw() {
    let events = load("text-reply-stream-300cps");
    let raw: Vec<&str> = events
        .iter()
        .filter_map(|e| match &e.data {
            EventData::Raw { kind, .. } => Some(kind.as_str()),
            _ => None,
        })
        .collect();
    assert!(raw.contains(&"provider.updated"));
    assert!(raw.contains(&"session.usage.updated"));
    assert!(raw.contains(&"session.step.streamed"));
    assert!(!raw.contains(&"session.metadata.updated"));
}

#[test]
fn metadata_updates_carry_the_work_state() {
    let events = load("agent-20tools-300cps");
    let states: Vec<_> = events
        .iter()
        .filter_map(|e| match &e.data {
            EventData::SessionMetadataUpdated(update) => Some(update.metadata.work_state()),
            _ => None,
        })
        .collect();
    assert!(!states.is_empty());
    assert!(states.contains(&Some(crate::types::WorkState::Open)));
}

#[test]
fn agent_turn_carries_tool_names_and_one_step_per_message() {
    let events = load("agent-20tools-300cps");
    assert_eq!(
        count(&events, |d| matches!(d, EventData::StepStarted(_))),
        21
    );
    assert_eq!(count(&events, |d| matches!(d, EventData::StepEnded(_))), 21);
    let names: Vec<&str> = events
        .iter()
        .filter_map(|e| match &e.data {
            EventData::ToolInputStarted(t) => Some(t.name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["glob"; 20]);
    assert_eq!(
        count(&events, |d| matches!(d, EventData::ToolCalled(_))),
        20
    );
    assert_eq!(
        count(&events, |d| matches!(d, EventData::ToolSuccess(_))),
        20
    );
}

#[test]
fn step_started_model_is_an_object() {
    let events = load("agent-20tools-300cps");
    let step = events
        .iter()
        .find_map(|e| match &e.data {
            EventData::StepStarted(s) => Some(s),
            _ => None,
        })
        .expect("step");
    assert_eq!(step.model.id, "agent-20tools-300cps");
    assert_eq!(step.agent, "build");
}

#[test]
fn interrupted_turn_reports_the_reason() {
    let events = load("interrupted-agent-turn");
    let reason = events
        .iter()
        .find_map(|e| match &e.data {
            EventData::ExecutionInterrupted(i) => Some(i.reason.as_deref()),
            _ => None,
        })
        .expect("interrupted");
    assert_eq!(reason, Some("user"));
}

#[test]
fn permission_ids_pair_up() {
    let events = load("permission-asked-agent-turn");
    let asked = events.iter().find_map(|e| match &e.data {
        EventData::PermissionAsked(p) => Some(p.id.clone()),
        _ => None,
    });
    let replied = events.iter().find_map(|e| match &e.data {
        EventData::PermissionReplied(p) => Some(p.request_id.clone()),
        _ => None,
    });
    assert!(asked.is_some());
    assert_eq!(asked, replied);
}

#[test]
fn session_lifecycle_has_created_and_deleted() {
    let events = load("session-lifecycle");
    let created = events.iter().find_map(|e| match &e.data {
        EventData::SessionCreated(c) => Some(c),
        _ => None,
    });
    let created = created.expect("created");
    assert!(created.location.directory.ends_with("/project"));
    assert_eq!(
        count(&events, |d| matches!(d, EventData::SessionDeleted(_))),
        1
    );
}

#[test]
fn rename_sets_the_title() {
    let events = load("text-reply-stream-300cps");
    assert_eq!(
        count(&events, |d| matches!(d, EventData::SessionRenamed(_))),
        1
    );
}

#[test]
fn server_connected_has_no_created() {
    let events = load("session-lifecycle");
    assert_eq!(events[0].created, None);
    assert!(events[1].created.is_some());
}

#[test]
fn shapes_never_captured_decode_from_hand_built_payloads() {
    let frames = [
        r#"{"id":"e1","created":1,"type":"session.status","data":{"sessionID":"ses_1","status":{"type":"busy"}}}"#,
        r#"{"id":"e2","created":1,"type":"session.idle","data":{"sessionID":"ses_1"}}"#,
        r#"{"id":"e3","created":1,"type":"session.execution.failed","data":{"sessionID":"ses_1","error":{"type":"x"}}}"#,
        r#"{"id":"e4","created":1,"type":"session.tool.failed","data":{"sessionID":"ses_1","assistantMessageID":"msg_1","id":"call_1","error":{"type":"x"}}}"#,
        r#"{"id":"e5","created":1,"type":"form.created","data":{"form":{"id":"frm_1","sessionID":"ses_1","fields":[]}}}"#,
        r#"{"id":"e6","created":1,"type":"form.replied","data":{"id":"frm_1","sessionID":"ses_1"}}"#,
        r#"{"id":"e7","created":1,"type":"form.cancelled","data":{"id":"frm_1","sessionID":"ses_1"}}"#,
    ];
    for frame in frames {
        let event: Event =
            serde_json::from_str(frame).unwrap_or_else(|err| panic!("{frame}: {err}"));
        assert!(!matches!(event.data, EventData::Raw { .. }), "{frame}");
    }
}

#[test]
fn unknown_type_is_kept_raw_with_its_data() {
    let event: Event =
        serde_json::from_str(r#"{"id":"e","created":2,"type":"future.thing","data":{"a":1}}"#)
            .expect("decode");
    assert_eq!(
        event.data,
        EventData::Raw {
            kind: "future.thing".into(),
            data: serde_json::json!({"a": 1}),
        }
    );
}

#[test]
fn a_known_type_with_a_broken_payload_is_an_error() {
    let result = serde_json::from_str::<Event>(
        r#"{"id":"e","created":2,"type":"session.text.delta","data":{"sessionID":"ses_1"}}"#,
    );
    assert!(result.is_err());
}
