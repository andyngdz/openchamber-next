use serde_json::Value;

use super::{CreateSessionRequest, SessionRecord, SwitchAgentRequest, SwitchModelRequest};
use crate::types::{Location, ModelRef};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn value(name: &str) -> Value {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

fn model(id: &str, provider: &str, variant: Option<&str>) -> ModelRef {
    ModelRef {
        id: id.to_owned(),
        provider_id: provider.to_owned(),
        variant: variant.map(str::to_owned),
    }
}

#[test]
fn the_captured_session_record_carries_its_agent_and_model() {
    let record: SessionRecord =
        serde_json::from_value(value("session-get.json")["data"].clone()).expect("record");

    assert_eq!(record.agent.as_deref(), Some("capture-primary"));
    let model = record.model.expect("model");
    assert_eq!(model.provider_id, "perf-second");
    assert_eq!(model.id, "alt-thinking");
    assert_eq!(model.variant.as_deref(), Some("medium"));
}

#[test]
fn a_create_with_an_agent_and_a_variant_serializes_to_the_captured_body() {
    let captured = value("session-create-agent-model.request.json");
    let request = CreateSessionRequest {
        location: Location {
            directory: captured["location"]["directory"]
                .as_str()
                .expect("directory")
                .to_owned(),
        },
        agent: Some("capture-primary".to_owned()),
        model: Some(model("alt-thinking", "perf-second", Some("medium"))),
    };

    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
}

#[test]
fn a_create_on_default_thinking_leaves_the_variant_out() {
    let captured = value("session-create-agent-model-default.request.json");
    let request = CreateSessionRequest {
        location: Location {
            directory: captured["location"]["directory"]
                .as_str()
                .expect("directory")
                .to_owned(),
        },
        agent: Some("capture-secondary".to_owned()),
        model: Some(model("stream-300cps", "perf", None)),
    };

    assert_eq!(serde_json::to_value(&request).expect("serialize"), captured);
}

#[test]
fn the_captured_model_switches_serialize_without_a_variant_for_default() {
    let with_variant = SwitchModelRequest {
        model: model("stream-600cps", "perf", Some("high")),
    };
    assert_eq!(
        serde_json::to_value(&with_variant).expect("serialize"),
        value("switch-model-variant.request.json")
    );

    let default = SwitchModelRequest {
        model: model("stream-300cps", "perf", None),
    };
    assert_eq!(
        serde_json::to_value(&default).expect("serialize"),
        value("switch-model-default.request.json")
    );
}

#[test]
fn the_captured_model_switch_without_a_variant_decodes() {
    let request: SwitchModelRequest =
        serde_json::from_value(value("switch-model.request.json")).expect("switch model");

    assert_eq!(request.model.id, "agent-20tools-300cps");
    assert_eq!(request.model.variant, None);
    assert_eq!(
        serde_json::to_value(&request).expect("serialize"),
        value("switch-model.request.json")
    );
}

#[test]
fn the_captured_agent_switch_serializes() {
    let request = SwitchAgentRequest {
        agent: "build".to_owned(),
    };

    assert_eq!(
        serde_json::to_value(&request).expect("serialize"),
        value("switch-agent.request.json")
    );
}
