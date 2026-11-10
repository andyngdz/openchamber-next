//! Picking an agent: the live pick, the per-session memory, pins and defaults.

use oc_client::catalog::AgentMode;
use oc_client::settings::Settings;
use serde_json::json;

use super::super::cascade::ProjectDefaults;
use super::super::memory::SelectionMemory;
use super::super::value::{EffortChoice, ModelKey, ModelPick, Selection, SelectionSource};
use super::{AgentContext, SessionScope, resolve_agent_selection};
use crate::fixture_test::{
    agent, catalog_with, chat_model, config, model_ref, primary_agent, provider, settings,
};

fn key(provider_id: &str, id: &str) -> ModelKey {
    ModelKey::new(provider_id, id)
}

fn base_catalog(pin: Option<oc_client::types::ModelRef>) -> crate::selection::Catalog {
    catalog_with(
        vec![provider("p"), provider("q")],
        vec![
            chat_model("p", "big-variant", &["low", "high"]),
            chat_model("p", "small", &[]),
            chat_model("q", "first", &["fast"]),
        ],
        vec![
            primary_agent("build"),
            agent("plan", AgentMode::Primary, false, pin),
            agent("plain", AgentMode::Primary, false, None),
        ],
        config(None, None),
    )
}

fn current(model: Option<(&str, &str)>, source: SelectionSource, pick: ModelPick) -> Selection {
    Selection {
        agent: Some("build".to_owned()),
        model: model.map(|(provider_id, id)| key(provider_id, id)),
        variant: None,
        variant_override: None,
        agent_source: SelectionSource::Auto,
        model_source: source,
        model_pick: pick,
    }
}

fn resolve(
    settings: &Settings,
    catalog: &crate::selection::Catalog,
    current: &Selection,
    memory: &SelectionMemory,
    scope: SessionScope,
    agent_name: &str,
) -> super::AgentOutcome {
    resolve_agent_selection(
        AgentContext {
            settings,
            project: ProjectDefaults::default(),
            catalog: Some(catalog),
            memory,
            scope,
        },
        current,
        agent_name,
    )
}

#[test]
fn a_live_model_pick_outranks_the_memory_and_stays() {
    let settings = settings(json!({}));
    let catalog = base_catalog(None);
    let mut memory = SelectionMemory::default();
    memory.remember_model("plan", &key("q", "first"));
    let current = current(
        Some(("p", "small")),
        SelectionSource::Manual,
        ModelPick::Live(key("p", "small")),
    );
    let outcome = resolve(
        &settings,
        &catalog,
        &current,
        &memory,
        SessionScope::Session,
        "plan",
    );
    assert_eq!(outcome.model, Some(key("p", "small")));
    assert!(outcome.remember_model);
}

#[test]
fn the_model_remembered_for_an_agent_is_restored() {
    let settings = settings(json!({}));
    let catalog = base_catalog(None);
    let mut memory = SelectionMemory::default();
    memory.remember_model("build", &key("p", "small"));
    let current = current(
        Some(("q", "first")),
        SelectionSource::Manual,
        ModelPick::None,
    );
    let outcome = resolve(
        &settings,
        &catalog,
        &current,
        &memory,
        SessionScope::Session,
        "build",
    );
    assert_eq!(outcome.model, Some(key("p", "small")));
    assert_eq!(outcome.model_source, SelectionSource::Manual);
}

#[test]
fn an_agents_pinned_model_beats_a_manual_pair() {
    let settings = settings(json!({}));
    let catalog = base_catalog(Some(model_ref("q", "first", Some("fast"))));
    let current = current(
        Some(("p", "small")),
        SelectionSource::Manual,
        ModelPick::None,
    );
    let outcome = resolve(
        &settings,
        &catalog,
        &current,
        &SelectionMemory::default(),
        SessionScope::Session,
        "plan",
    );
    assert_eq!(outcome.model, Some(key("q", "first")));
    assert_eq!(
        outcome.variant,
        Some(EffortChoice::Named("fast".to_owned()))
    );
    assert_eq!(outcome.model_source, SelectionSource::Auto);
}

#[test]
fn a_manual_pair_is_kept_when_the_agent_pins_nothing() {
    let settings = settings(json!({"defaultModel": "q/first"}));
    let catalog = base_catalog(None);
    let current = current(
        Some(("p", "small")),
        SelectionSource::Manual,
        ModelPick::None,
    );
    let outcome = resolve(
        &settings,
        &catalog,
        &current,
        &SelectionMemory::default(),
        SessionScope::Session,
        "plain",
    );
    assert_eq!(outcome.model, Some(key("p", "small")));
    assert!(outcome.remember_model);
}

#[test]
fn the_settings_default_applies_when_nothing_else_names_a_model() {
    let settings = settings(json!({"defaultModel": "q/first", "defaultVariant": "fast"}));
    let catalog = base_catalog(None);
    let current = current(Some(("p", "small")), SelectionSource::Auto, ModelPick::None);
    let outcome = resolve(
        &settings,
        &catalog,
        &current,
        &SelectionMemory::default(),
        SessionScope::Session,
        "plain",
    );
    assert_eq!(outcome.model, Some(key("q", "first")));
    assert_eq!(
        outcome.variant,
        Some(EffortChoice::Named("fast".to_owned()))
    );
}

#[test]
fn the_current_model_survives_when_no_step_names_another() {
    let settings = settings(json!({}));
    let catalog = base_catalog(None);
    let current = current(Some(("p", "small")), SelectionSource::Auto, ModelPick::None);
    let outcome = resolve(
        &settings,
        &catalog,
        &current,
        &SelectionMemory::default(),
        SessionScope::Session,
        "plain",
    );
    assert_eq!(outcome.model, Some(key("p", "small")));
    assert!(!outcome.remember_model);
}

#[test]
fn a_saved_session_variant_outranks_the_agent_pin() {
    let settings = settings(json!({}));
    let catalog = base_catalog(Some(model_ref("q", "first", Some("fast"))));
    let mut memory = SelectionMemory::default();
    memory.remember_model("build", &key("q", "first"));
    memory.remember_variant("build", &key("q", "first"), Some(&EffortChoice::Default));
    let current = current(
        Some(("p", "small")),
        SelectionSource::Manual,
        ModelPick::None,
    );
    let outcome = resolve(
        &settings,
        &catalog,
        &current,
        &memory,
        SessionScope::Session,
        "build",
    );
    assert_eq!(outcome.model, Some(key("q", "first")));
    assert_eq!(outcome.variant, Some(EffortChoice::Default));
    assert_eq!(outcome.variant_override, Some(EffortChoice::Default));
}

#[test]
fn the_variant_order_is_agent_pin_then_configured_then_remembered() {
    let settings = settings(json!({"lastUsedEfforts": {"p/big-variant": "low"}}));
    let catalog = base_catalog(None);
    let mut memory = SelectionMemory::default();
    memory.remember_model("build", &key("p", "big-variant"));
    let current = current(None, SelectionSource::Auto, ModelPick::None);
    let outcome = resolve(
        &settings,
        &catalog,
        &current,
        &memory,
        SessionScope::Session,
        "build",
    );
    assert_eq!(outcome.variant, Some(EffortChoice::Named("low".to_owned())));

    let pinned = base_catalog(Some(model_ref("p", "big-variant", Some("high"))));
    let mut memory = SelectionMemory::default();
    memory.remember_model("plan", &key("p", "big-variant"));
    let outcome = resolve(
        &settings,
        &pinned,
        &current,
        &memory,
        SessionScope::Session,
        "plan",
    );
    assert_eq!(
        outcome.variant,
        Some(EffortChoice::Named("high".to_owned()))
    );
}

#[test]
fn a_drafts_live_override_for_the_same_model_is_kept() {
    let settings = settings(json!({}));
    let catalog = base_catalog(None);
    let mut current = current(
        Some(("p", "big-variant")),
        SelectionSource::Auto,
        ModelPick::None,
    );
    current.variant_override = Some(EffortChoice::Named("low".to_owned()));
    let mut memory = SelectionMemory::default();
    memory.remember_model("plain", &key("p", "big-variant"));
    let outcome = resolve(
        &settings,
        &catalog,
        &current,
        &memory,
        SessionScope::Draft,
        "plain",
    );
    assert_eq!(outcome.model, Some(key("p", "big-variant")));
    assert_eq!(outcome.variant, Some(EffortChoice::Named("low".to_owned())));
}
