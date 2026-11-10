use serde_json::Value;

use super::{
    Agent, AgentMode, ConfigModel, ConfigSource, Model, ModelKind, OpenCodeConfig, Provider,
    ProviderActivation,
};
use crate::types::Located;

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn value(name: &str) -> Value {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

#[test]
fn the_captured_providers_decode_with_their_activation() {
    let located: Located<Vec<Provider>> =
        serde_json::from_value(value("providers.json")).expect("providers");

    assert_eq!(located.data.len(), 3);
    assert!(
        located
            .data
            .iter()
            .all(|provider| provider.activation == ProviderActivation::Enabled)
    );
    assert_eq!(located.data[1].id, "perf");
    assert_eq!(located.data[1].name, "Performance fixture");
}

#[test]
fn the_captured_models_decode_with_variants_and_kinds() {
    let located: Located<Vec<Model>> =
        serde_json::from_value(value("models.json")).expect("models");

    assert!(located.data.len() >= 30);
    let thinking = located
        .data
        .iter()
        .find(|model| model.id == "stream-600cps")
        .expect("stream-600cps");
    assert_eq!(thinking.provider_id, "perf");
    assert!(thinking.enabled);
    assert_eq!(
        thinking
            .variants
            .iter()
            .map(|variant| variant.id.as_str())
            .collect::<Vec<_>>(),
        ["low", "high"]
    );
    let fixture = located
        .data
        .iter()
        .find(|model| model.id == "stream-300cps")
        .expect("stream-300cps");
    assert_eq!(fixture.kind, ModelKind::Chat, "an unset type is chat");
    let pickle = located
        .data
        .iter()
        .find(|model| model.id == "big-pickle")
        .expect("big-pickle");
    assert_eq!(pickle.kind, ModelKind::Chat);
}

#[test]
fn the_captured_agents_decode_with_mode_hidden_and_pin() {
    let located: Located<Vec<Agent>> =
        serde_json::from_value(value("agents.json")).expect("agents");

    assert_eq!(located.data.len(), 11);
    let hidden = located
        .data
        .iter()
        .find(|agent| agent.id == "capture-hidden")
        .expect("capture-hidden");
    assert!(hidden.hidden);
    let subagent = located
        .data
        .iter()
        .find(|agent| agent.id == "capture-subagent")
        .expect("capture-subagent");
    assert_eq!(subagent.mode, AgentMode::Subagent);
    let pinned = located
        .data
        .iter()
        .find(|agent| agent.id == "capture-primary")
        .expect("capture-primary");
    let model = pinned.model.as_ref().expect("pinned model");
    assert_eq!(model.provider_id, "perf");
    assert_eq!(model.id, "stream-300cps");

    let build = located
        .data
        .iter()
        .find(|agent| agent.id == "build")
        .expect("build");
    assert_eq!(build.display_name, "Build");
    assert!(build.description.starts_with("The default agent"));
}

#[test]
fn the_captured_config_sources_fold_to_the_opencode_defaults() {
    let sources: Vec<ConfigSource> =
        serde_json::from_value(value("config.json")).expect("config sources");
    let mut folded = serde_json::Map::new();
    for source in sources {
        if let Some(Value::Object(info)) = source.info {
            folded.extend(info);
        }
    }
    let config: OpenCodeConfig =
        serde_json::from_value(Value::Object(folded)).expect("folded config");

    assert_eq!(config.default_agent.as_deref(), Some("capture-secondary"));
    let model = config.model.expect("model");
    assert_eq!(model.provider_id, "perf");
    assert_eq!(model.model, "stream-300cps");
}

#[test]
fn a_config_model_under_either_key_decodes() {
    let under_model: ConfigModel =
        serde_json::from_value(value("config.json")[2]["info"]["model"].clone()).expect("model");
    let under_model_id: ConfigModel =
        serde_json::from_str(r#"{"providerID":"perf","modelID":"stream-300cps","variant":"high"}"#)
            .expect("modelID");

    assert_eq!(under_model.model, "stream-300cps");
    assert_eq!(under_model_id.model, "stream-300cps");
    assert_eq!(under_model_id.variant.as_deref(), Some("high"));
}
