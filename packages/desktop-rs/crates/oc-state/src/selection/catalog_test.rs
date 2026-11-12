//! Building the catalog: provider, model and agent rules, from the unit 1 fixtures.

use oc_client::catalog::{AgentMode, ModelKind, ProviderActivation};

use super::super::value::ModelKey;
use super::Catalog;
use crate::fixture_test::{
    agent, api_catalog, api_models, catalog_with, chat_model, config, model, primary_agent,
    provider,
};

#[test]
fn the_fixture_catalog_keeps_three_providers_and_their_chat_models() {
    let catalog = api_catalog();
    let ids: Vec<&str> = catalog
        .providers()
        .iter()
        .map(|provider| provider.provider.id.as_str())
        .collect();
    assert_eq!(ids, ["opencode", "perf", "perf-second"]);

    let counts: Vec<usize> = catalog
        .providers()
        .iter()
        .map(|provider| provider.models.len())
        .collect();
    let models = api_models();
    let expected: Vec<usize> = ids
        .iter()
        .map(|id| {
            models
                .iter()
                .filter(|model| {
                    model.provider_id == *id && model.enabled && model.kind != ModelKind::Image
                })
                .count()
        })
        .collect();
    assert_eq!(counts, expected);

    let key = ModelKey::new("perf", "stream-600cps");
    let model = catalog.model(&key).expect("stream-600cps");
    let variants: Vec<&str> = model.variants.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(variants, ["low", "high"]);
    assert!(catalog.has_model(&key));
    assert!(catalog.model(&ModelKey::new("perf", "missing")).is_none());
}

#[test]
fn the_fixture_agent_picker_is_primary_and_sorted_by_lowercased_name() {
    let catalog = api_catalog();
    let ids: Vec<&str> = catalog
        .picker_agents()
        .iter()
        .map(|agent| agent.id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["build", "capture-primary", "capture-secondary", "plan"]
    );
}

#[test]
fn a_switched_off_provider_or_an_image_only_one_is_not_offered() {
    let mut disabled = provider("off");
    disabled.activation = ProviderActivation::Disabled;
    let catalog = catalog_with(
        vec![disabled, provider("on")],
        vec![chat_model("off", "m", &[])],
        vec![],
        config(None, None),
    );
    assert!(catalog.providers().is_empty());

    let catalog = catalog_with(
        vec![provider("images")],
        vec![model("images", "generator", &[], true, ModelKind::Image)],
        vec![],
        config(None, None),
    );
    assert!(catalog.providers().is_empty());
}

#[test]
fn disabled_and_image_models_are_left_out_of_an_offered_provider() {
    let catalog = catalog_with(
        vec![provider("p")],
        vec![
            chat_model("p", "chat", &[]),
            model("p", "disabled", &[], false, ModelKind::Chat),
            model("p", "image", &[], true, ModelKind::Image),
        ],
        vec![],
        config(None, None),
    );
    let ids: Vec<&str> = catalog.providers()[0]
        .models
        .iter()
        .map(|model| model.id.as_str())
        .collect();
    assert_eq!(ids, ["chat"]);
}

#[test]
fn subagents_and_hidden_agents_stay_out_of_the_picker() {
    let catalog = catalog_with(
        vec![],
        vec![],
        vec![
            primary_agent("build"),
            agent("hidden", AgentMode::Primary, true, None),
            agent("sub", AgentMode::Subagent, false, None),
            agent("other", AgentMode::Other, false, None),
            primary_agent("alpha"),
        ],
        config(None, None),
    );
    assert_eq!(catalog.agents().len(), 5);
    let ids: Vec<&str> = catalog
        .picker_agents()
        .iter()
        .map(|agent| agent.id.as_str())
        .collect();
    assert_eq!(ids, ["alpha", "build"]);
}

#[test]
fn an_all_mode_agent_is_offered_and_sorted() {
    let mut all = primary_agent("zulu");
    all.mode = AgentMode::All;
    let catalog = Catalog::from_parts(
        Vec::new(),
        Vec::new(),
        vec![primary_agent("beta"), all],
        config(None, None),
    );
    assert_eq!(
        catalog
            .picker_agents()
            .iter()
            .map(|agent| agent.id.as_str())
            .collect::<Vec<_>>(),
        ["beta", "zulu"]
    );
}
