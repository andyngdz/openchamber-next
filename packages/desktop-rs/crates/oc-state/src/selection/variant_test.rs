//! The inherited Thinking level and the session/draft override.

use oc_client::catalog::AgentMode;
use serde_json::json;

use super::super::cascade::ProjectDefaults;
use super::super::memory::SelectionMemory;
use super::super::set_agent::{AgentContext, SessionScope};
use super::super::value::{EffortChoice, ModelKey, ModelPick, Selection, SelectionSource};
use super::{inherited_variant, resolved_variant};
use crate::fixture_test::{
    agent, catalog_with, chat_model, config, model_ref, primary_agent, provider, settings,
};

fn key(provider_id: &str, id: &str) -> ModelKey {
    ModelKey::new(provider_id, id)
}

fn catalog(pin: Option<oc_client::types::ModelRef>) -> crate::selection::Catalog {
    catalog_with(
        vec![provider("p")],
        vec![chat_model("p", "think", &["low", "high"])],
        vec![
            primary_agent("build"),
            agent("plan", AgentMode::Primary, false, pin),
        ],
        config(None, None),
    )
}

fn context<'a>(
    settings: &'a oc_client::settings::Settings,
    catalog: &'a crate::selection::Catalog,
    memory: &'a SelectionMemory,
    scope: SessionScope,
) -> AgentContext<'a> {
    AgentContext {
        settings,
        project: ProjectDefaults::default(),
        catalog: Some(catalog),
        memory,
        scope,
    }
}

#[test]
fn the_pin_then_the_configured_default_then_the_remembered_effort() {
    let plain = catalog(None);
    let configured = settings(json!({"defaultModel": "p/think", "defaultVariant": "high"}));
    let build = plain.agent("build");
    assert_eq!(
        inherited_variant(
            context(
                &configured,
                &plain,
                &SelectionMemory::default(),
                SessionScope::Session
            ),
            build,
            &key("p", "think")
        ),
        Some(EffortChoice::Named("high".to_owned()))
    );

    let pinned = catalog(Some(model_ref("p", "think", Some("low"))));
    let plan = pinned.agent("plan");
    assert_eq!(
        inherited_variant(
            context(
                &configured,
                &pinned,
                &SelectionMemory::default(),
                SessionScope::Session
            ),
            plan,
            &key("p", "think")
        ),
        Some(EffortChoice::Named("low".to_owned()))
    );

    let remembered = settings(json!({"lastUsedEfforts": {"p/think": "high"}}));
    assert_eq!(
        inherited_variant(
            context(
                &remembered,
                &plain,
                &SelectionMemory::default(),
                SessionScope::Session
            ),
            build,
            &key("p", "think")
        ),
        Some(EffortChoice::Named("high".to_owned()))
    );
}

#[test]
fn an_explicit_default_in_the_memory_stops_the_search() {
    let plain = catalog(None);
    let settings = settings(json!({"lastUsedEfforts": {"p/think": "high"}}));
    let mut memory = SelectionMemory::default();
    memory.remember_variant("build", &key("p", "think"), Some(&EffortChoice::Default));
    let current = Selection {
        agent: Some("build".to_owned()),
        model: Some(key("p", "think")),
        variant: None,
        variant_override: None,
        agent_source: SelectionSource::Auto,
        model_source: SelectionSource::Manual,
        model_pick: ModelPick::None,
    };
    let (effective, explicit) = resolved_variant(
        context(&settings, &plain, &memory, SessionScope::Session),
        plain.agent("build"),
        "build",
        &key("p", "think"),
        &current,
    );
    assert_eq!(effective, Some(EffortChoice::Default));
    assert_eq!(explicit, Some(EffortChoice::Default));
}

#[test]
fn a_model_without_variants_has_no_level_and_a_model_outside_the_catalog_is_left_unchecked() {
    let none = catalog_with(
        vec![provider("p")],
        vec![chat_model("p", "plain", &[])],
        vec![primary_agent("build")],
        config(None, None),
    );
    let settings = settings(json!({}));
    let current = Selection::default();
    assert_eq!(
        resolved_variant(
            context(
                &settings,
                &none,
                &SelectionMemory::default(),
                SessionScope::Draft
            ),
            none.agent("build"),
            "build",
            &key("p", "plain"),
            &current
        ),
        (None, None)
    );

    let outside = key("gone", "model");
    assert_eq!(
        inherited_variant(
            context(
                &settings,
                &none,
                &SelectionMemory::default(),
                SessionScope::Draft
            ),
            none.agent("build"),
            &outside
        ),
        None
    );
}
