use super::{Event, EventData};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/events");
const API_FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

const FILES: [&str; 7] = [
    "text-reply-stream-300cps",
    "agent-20tools-300cps",
    "interrupted-agent-turn",
    "permission-asked-agent-turn",
    "two-sessions-stream-300cps",
    "session-lifecycle",
    "turn-control",
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

fn frames(name: &str) -> Vec<serde_json::Value> {
    let path = format!("{FIXTURES}/{name}.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("{path}: {err}"))
}

/// The agent the capture's sessions start on when they name none themselves.
fn configured_default_agent() -> String {
    let path = format!("{API_FIXTURES}/config.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    let sources: Vec<serde_json::Value> =
        serde_json::from_str(&text).unwrap_or_else(|err| panic!("{path}: {err}"));
    sources
        .iter()
        .filter_map(|source| source["info"]["default_agent"].as_str())
        .next_back()
        .expect("the capture config names a default agent")
        .to_owned()
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
    let first_input = events.iter().find_map(|e| match &e.data {
        EventData::ToolCalled(called) => Some(&called.input),
        _ => None,
    });
    assert_eq!(
        first_input,
        Some(&serde_json::json!({"pattern": "packages/*/package.json"}))
    );
    assert_eq!(
        count(&events, |d| matches!(d, EventData::ToolSuccess(_))),
        20
    );
}

#[test]
fn step_started_model_is_an_object() {
    let events = load("agent-20tools-300cps");
    // The scenario's step, chosen by its model id rather than by position: the file also
    // carries the session's own event traffic.
    let step = events
        .iter()
        .find_map(|e| match &e.data {
            EventData::StepStarted(s) if s.model.id == "agent-20tools-300cps" => Some(s),
            _ => None,
        })
        .expect("the agent-20tools step");
    assert_eq!(step.model.id, "agent-20tools-300cps");
    assert_eq!(step.model.provider_id, "perf");
    assert_eq!(step.agent, configured_default_agent());
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
fn permission_asked_carries_the_whole_request() {
    let events = load("turn-control");
    let asked = events
        .iter()
        .find_map(|e| match &e.data {
            EventData::PermissionAsked(asked) => Some(asked),
            _ => None,
        })
        .expect("the turn-control run asks");
    let raw = frames("turn-control")
        .into_iter()
        .find(|frame| frame["type"] == "permission.asked")
        .map(|frame| frame["data"].clone())
        .expect("the captured permission.asked");

    assert_eq!(asked.id, raw["id"].as_str().expect("id"));
    assert_eq!(
        asked.session_id,
        raw["sessionID"].as_str().expect("session")
    );
    assert_eq!(
        asked.request.action,
        raw["action"].as_str().expect("action")
    );
    assert_eq!(
        asked.request.resources.len(),
        raw["resources"].as_array().expect("resources").len()
    );
    assert_eq!(asked.id, asked.request.id);
    let source = asked.request.source.as_ref().expect("a tool source");
    let raw_source = &raw["source"];
    assert_eq!(
        source.message_id,
        raw_source["messageID"].as_str().expect("message")
    );
    assert_eq!(source.id, raw_source["id"].as_str().expect("call"));
}

#[test]
fn form_created_carries_the_whole_form() {
    let events = load("turn-control");
    let created = events
        .iter()
        .find_map(|e| match &e.data {
            EventData::FormCreated(created) => Some(created),
            _ => None,
        })
        .expect("the turn-control run asks a question");
    let raw = frames("turn-control")
        .into_iter()
        .find(|frame| frame["type"] == "form.created")
        .map(|frame| frame["data"]["form"].clone())
        .expect("the captured form.created");

    assert_eq!(created.form.id, raw["id"].as_str().expect("id"));
    assert_eq!(
        created.form.session_id,
        raw["sessionID"].as_str().expect("session")
    );
    assert_eq!(created.form.title, raw["title"].as_str().expect("title"));
    assert_eq!(
        created.form.fields.len(),
        raw["fields"].as_array().expect("fields").len()
    );
    assert_eq!(created.form.metadata["kind"], raw["metadata"]["kind"]);
}

#[test]
fn hand_built_events_naming_only_their_ids_still_decode() {
    let asked: Event = serde_json::from_str(
        r#"{"id":"e1","created":1,"type":"permission.asked","data":{"id":"per_1","sessionID":"ses_1"}}"#,
    )
    .expect("bare permission.asked");
    let EventData::PermissionAsked(asked) = asked.data else {
        panic!("permission.asked expected");
    };
    assert_eq!(asked.id, "per_1");
    assert_eq!(asked.request.session_id, "ses_1");
    assert!(asked.request.resources.is_empty());

    let created: Event = serde_json::from_str(
        r#"{"id":"e2","created":1,"type":"form.created","data":{"form":{"id":"frm_1","sessionID":"ses_1"}}}"#,
    )
    .expect("bare form.created");
    let EventData::FormCreated(created) = created.data else {
        panic!("form.created expected");
    };
    assert_eq!(created.form.id, "frm_1");
    assert!(created.form.fields.is_empty());
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
