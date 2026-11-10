//! The default agent and model cascade, one test per step.

use oc_client::catalog::AgentMode;
use serde_json::json;

use super::super::value::{EffortChoice, ModelKey};
use super::{ProjectDefaults, resolve_default_selection};
use crate::fixture_test::{
    agent, catalog_with, chat_model, config, model_ref, primary_agent, provider, settings,
};

fn empty_project<'a>() -> ProjectDefaults<'a> {
    ProjectDefaults::default()
}

fn catalog() -> crate::selection::Catalog {
    catalog_with(
        vec![provider("p"), provider("q")],
        vec![
            chat_model("p", "big-variant", &["low", "high"]),
            chat_model("p", "small", &[]),
            chat_model("q", "first", &["fast"]),
        ],
        vec![
            primary_agent("build"),
            primary_agent("plan"),
            agent("sub", AgentMode::Subagent, false, None),
        ],
        config(None, None),
    )
}

fn key(provider_id: &str, id: &str) -> ModelKey {
    ModelKey::new(provider_id, id)
}

fn named(value: &str) -> Option<EffortChoice> {
    Some(EffortChoice::Named(value.to_owned()))
}

#[test]
fn the_project_default_agent_and_model_win() {
    let settings = settings(json!({
        "defaultAgent": "plan",
        "defaultModel": "q/first",
        "defaultVariant": "fast",
    }));
    let project = ProjectDefaults {
        agent: Some("build"),
        model: Some("p/big-variant"),
        variant: Some("high"),
    };
    let selection = resolve_default_selection(&settings, project, Some(&catalog()));
    assert_eq!(selection.agent(), Some("build"));
    assert_eq!(selection.model(), Some(&key("p", "big-variant")));
    assert_eq!(selection.variant(), named("high").as_ref());
}

#[test]
fn the_settings_default_agent_and_model_apply_when_the_project_names_none() {
    let settings = settings(json!({
        "defaultAgent": "plan",
        "defaultModel": "q/first",
        "defaultVariant": "fast",
    }));
    let selection = resolve_default_selection(&settings, empty_project(), Some(&catalog()));
    assert_eq!(selection.agent(), Some("plan"));
    assert_eq!(selection.model(), Some(&key("q", "first")));
    assert_eq!(selection.variant(), named("fast").as_ref());
}

#[test]
fn the_opencode_default_agent_counts_only_when_it_is_a_visible_primary_agent() {
    let settings = settings(json!({"defaultAgent": "missing"}));
    let with_plan = catalog_with(
        vec![],
        vec![],
        vec![primary_agent("build"), primary_agent("plan")],
        config(Some("plan"), None),
    );
    let selection = resolve_default_selection(&settings, empty_project(), Some(&with_plan));
    assert_eq!(selection.agent(), Some("plan"));

    let subagent = catalog_with(
        vec![],
        vec![],
        vec![
            primary_agent("build"),
            agent("sub", AgentMode::Subagent, false, None),
        ],
        config(Some("sub"), None),
    );
    let selection = resolve_default_selection(&settings, empty_project(), Some(&subagent));
    assert_eq!(selection.agent(), Some("build"));
}

#[test]
fn build_then_the_first_primary_then_the_first_agent() {
    let with_build = catalog_with(
        vec![],
        vec![],
        vec![primary_agent("alpha"), primary_agent("build")],
        config(None, None),
    );
    assert_eq!(
        resolve_default_selection(&settings(json!({})), empty_project(), Some(&with_build)).agent(),
        Some("build")
    );

    let without_build = catalog_with(
        vec![],
        vec![],
        vec![primary_agent("alpha"), primary_agent("zeta")],
        config(None, None),
    );
    assert_eq!(
        resolve_default_selection(&settings(json!({})), empty_project(), Some(&without_build))
            .agent(),
        Some("alpha")
    );

    let only_subagent = catalog_with(
        vec![],
        vec![],
        vec![agent("sub", AgentMode::Subagent, false, None)],
        config(None, None),
    );
    assert_eq!(
        resolve_default_selection(&settings(json!({})), empty_project(), Some(&only_subagent))
            .agent(),
        Some("sub")
    );
}

#[test]
fn an_agents_pinned_model_carries_its_variant() {
    let settings = settings(json!({"defaultAgent": "plan"}));
    let catalog = catalog_with(
        vec![provider("p")],
        vec![chat_model("p", "big-variant", &["low", "high"])],
        vec![agent(
            "plan",
            AgentMode::Primary,
            false,
            Some(model_ref("p", "big-variant", Some("low"))),
        )],
        config(None, None),
    );
    let selection = resolve_default_selection(&settings, empty_project(), Some(&catalog));
    assert_eq!(selection.agent(), Some("plan"));
    assert_eq!(selection.model(), Some(&key("p", "big-variant")));
    assert_eq!(selection.variant(), named("low").as_ref());
}

#[test]
fn the_opencode_config_model_applies_after_the_agent_pin() {
    let catalog = catalog_with(
        vec![provider("q")],
        vec![chat_model("q", "first", &["fast"])],
        vec![primary_agent("build")],
        config(None, Some(("q", "first", Some("fast")))),
    );
    let selection =
        resolve_default_selection(&settings(json!({})), empty_project(), Some(&catalog));
    assert_eq!(selection.model(), Some(&key("q", "first")));
    assert_eq!(selection.variant(), named("fast").as_ref());
}

#[test]
fn last_selected_model_then_big_pickle_then_the_first_visible_model() {
    let chosen = settings(json!({"lastSelectedModel": "p/small"}));
    let selection = resolve_default_selection(&chosen, empty_project(), Some(&catalog()));
    assert_eq!(selection.model(), Some(&key("p", "small")));

    let fallback = catalog_with(
        vec![provider("opencode"), provider("p")],
        vec![
            chat_model("opencode", "big-pickle", &[]),
            chat_model("p", "small", &[]),
        ],
        vec![primary_agent("build")],
        config(None, None),
    );
    let selection =
        resolve_default_selection(&settings(json!({})), empty_project(), Some(&fallback));
    assert_eq!(selection.model(), Some(&key("opencode", "big-pickle")));

    let skip_hidden = settings(json!({"hiddenModels": [{"providerID": "p", "modelID": "small"}]}));
    let selection = resolve_default_selection(&skip_hidden, empty_project(), Some(&catalog()));
    assert_eq!(selection.model(), Some(&key("p", "big-variant")));
}

#[test]
fn a_everything_hidden_catalog_still_offers_big_pickle_then_the_first_model() {
    let big_pickle_hidden = settings(json!({
        "hiddenModels": [{"providerID": "opencode", "modelID": "big-pickle"}]
    }));
    let catalog = catalog_with(
        vec![provider("opencode")],
        vec![chat_model("opencode", "big-pickle", &[])],
        vec![],
        config(None, None),
    );
    assert_eq!(
        resolve_default_selection(&big_pickle_hidden, empty_project(), Some(&catalog)).model(),
        Some(&key("opencode", "big-pickle"))
    );

    let all_hidden = settings(json!({"hiddenModels": [{"providerID": "p", "modelID": "small"}]}));
    let only_small = catalog_with(
        vec![provider("p")],
        vec![chat_model("p", "small", &[])],
        vec![],
        config(None, None),
    );
    assert_eq!(
        resolve_default_selection(&all_hidden, empty_project(), Some(&only_small)).model(),
        Some(&key("p", "small"))
    );
}

#[test]
fn a_configured_default_stays_selected_when_the_catalog_lacks_it() {
    let settings = settings(json!({"defaultModel": "gone/missing", "defaultVariant": "high"}));
    let selection = resolve_default_selection(&settings, empty_project(), None);
    assert_eq!(selection.model(), Some(&key("gone", "missing")));
    assert_eq!(selection.variant(), named("high").as_ref());

    let empty = catalog_with(vec![], vec![], vec![], config(None, None));
    let selection = resolve_default_selection(&settings, empty_project(), Some(&empty));
    assert_eq!(selection.model(), Some(&key("gone", "missing")));
    assert_eq!(selection.variant(), named("high").as_ref());
}

#[test]
fn an_absent_catalog_selects_no_agent_or_fallback_model() {
    let selection = resolve_default_selection(&settings(json!({})), empty_project(), None);
    assert_eq!(selection.agent(), None);
    assert_eq!(selection.model(), None);
}

#[test]
fn the_remembered_effort_fills_an_unset_variant_and_default_stops_the_search() {
    let remembered = settings(json!({"lastUsedEfforts": {"p/big-variant": "high"}}));
    let catalog = catalog_with(
        vec![provider("p")],
        vec![chat_model("p", "big-variant", &["low", "high"])],
        vec![agent(
            "plan",
            AgentMode::Primary,
            false,
            Some(model_ref("p", "big-variant", None)),
        )],
        config(None, None),
    );
    let selection = resolve_default_selection(
        &remembered,
        ProjectDefaults {
            agent: Some("plan"),
            model: None,
            variant: None,
        },
        Some(&catalog),
    );
    assert_eq!(selection.variant(), named("high").as_ref());

    let default = settings(json!({"lastUsedEfforts": {"p/big-variant": null}}));
    let selection = resolve_default_selection(
        &default,
        ProjectDefaults {
            agent: Some("plan"),
            model: None,
            variant: None,
        },
        Some(&catalog),
    );
    assert_eq!(selection.variant(), Some(&EffortChoice::Default));
}

#[test]
fn the_settings_variant_never_leaks_onto_a_project_model() {
    let settings = settings(json!({"defaultModel": "q/first", "defaultVariant": "fast"}));
    let project = ProjectDefaults {
        agent: None,
        model: Some("p/big-variant"),
        variant: None,
    };
    let selection = resolve_default_selection(&settings, project, Some(&catalog()));
    assert_eq!(selection.model(), Some(&key("p", "big-variant")));
    assert_eq!(selection.variant(), None);
}

#[test]
fn a_hidden_model_is_not_picked_by_last_selected_model() {
    let chosen = settings(json!({
        "lastSelectedModel": "p/small",
        "hiddenModels": [{"providerID": "p", "modelID": "small"}]
    }));
    let selection = resolve_default_selection(&chosen, empty_project(), Some(&catalog()));
    assert_eq!(selection.model(), Some(&key("p", "big-variant")));
}
