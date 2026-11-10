//! The model sections, the ranked agent list and the ranked project list.

use serde_json::json;

use super::{model_sections, ranked_agents, ranked_projects};
use crate::fixture_test::{
    api_catalog, api_settings, catalog_with, config, described_agent, settings,
};

fn ids(entries: &[super::ModelEntry]) -> Vec<&str> {
    entries
        .iter()
        .map(|entry| entry.model_id.as_str())
        .collect()
}

#[test]
fn the_fixture_sections_list_favorites_then_recent_then_providers() {
    let catalog = api_catalog();
    let sections = model_sections(&catalog, &api_settings(), "");
    assert_eq!(ids(&sections.favorites), ["stream-600cps", "alt-thinking"]);
    assert_eq!(ids(&sections.recent), ["alt-chat-300cps", "stream-300cps"]);
    let providers: Vec<(&str, usize)> = sections
        .providers
        .iter()
        .map(|section| (section.provider_id.as_str(), section.models.len()))
        .collect();
    assert_eq!(
        providers,
        [("opencode", 10), ("perf", 22), ("perf-second", 3)]
    );
    assert!(
        sections
            .providers
            .iter()
            .flat_map(|section| section.models.iter())
            .all(|entry| entry.model_id != "stream-100cps")
    );
    let favorite = &sections.favorites[0];
    assert_eq!(favorite.name, "Fixture stream, 600 characters/s");
    assert_eq!(favorite.provider_name, "Performance fixture");
    assert_eq!(favorite.variants, ["low", "high"]);
    assert_eq!(favorite.context, Some(200_000));
}

#[test]
fn provider_order_puts_the_named_providers_first_and_keeps_the_rest() {
    let catalog = api_catalog();
    let ordered = settings(json!({
        "providerOrder": ["perf-second", "opencode"]
    }));
    let sections = model_sections(&catalog, &ordered, "");
    let providers: Vec<&str> = sections
        .providers
        .iter()
        .map(|section| section.provider_id.as_str())
        .collect();
    assert_eq!(providers, ["perf-second", "opencode", "perf"]);
}

#[test]
fn the_search_filters_every_section_without_reordering_it() {
    let catalog = api_catalog();
    let settings = api_settings();
    let sections = model_sections(&catalog, &settings, "alt");
    assert_eq!(ids(&sections.favorites), ["alt-thinking"]);
    assert_eq!(ids(&sections.recent), ["alt-chat-300cps"]);
    let providers: Vec<&str> = sections
        .providers
        .iter()
        .map(|section| section.provider_id.as_str())
        .collect();
    assert_eq!(providers, ["perf-second"]);

    let by_provider = model_sections(&catalog, &settings, "performance fixture");
    assert!(
        by_provider
            .providers
            .iter()
            .all(|section| section.provider_id != "opencode")
    );
    assert_eq!(by_provider.providers.len(), 2);
    assert_eq!(
        ids(&by_provider.favorites),
        ["stream-600cps", "alt-thinking"]
    );

    let by_id = model_sections(&catalog, &settings, "stream-1200cps");
    assert!(by_id.favorites.is_empty());
    assert!(by_id.recent.is_empty());
    let found: Vec<&str> = by_id
        .providers
        .iter()
        .flat_map(|section| section.models.iter())
        .map(|entry| entry.model_id.as_str())
        .collect();
    assert_eq!(found, ["stream-1200cps"]);
}

#[test]
fn the_agent_list_ranks_name_and_display_name() {
    let catalog = api_catalog();
    let ranked: Vec<&str> = ranked_agents(&catalog, "cap")
        .iter()
        .map(|agent| agent.id.as_str())
        .collect();
    assert_eq!(ranked, ["capture-primary", "capture-secondary"]);
    let plan: Vec<&str> = ranked_agents(&catalog, "plan")
        .iter()
        .map(|agent| agent.id.as_str())
        .collect();
    assert_eq!(plan, ["plan"]);
    assert_eq!(ranked_agents(&catalog, "no-such-agent").len(), 0);
}

#[test]
fn the_project_list_ranks_label_and_path_and_keeps_settings_order_on_an_empty_query() {
    let settings = api_settings();
    let all: Vec<&str> = ranked_projects(&settings, "")
        .iter()
        .filter_map(|project| project.label())
        .collect();
    assert_eq!(all, ["Fixture project", "Second fixture project"]);

    let second: Vec<&str> = ranked_projects(&settings, "second")
        .iter()
        .filter_map(|project| project.label())
        .collect();
    assert_eq!(second, ["Second fixture project"]);

    let by_path: Vec<&str> = ranked_projects(&settings, "project-two")
        .iter()
        .filter_map(|project| project.label())
        .collect();
    assert_eq!(by_path, ["Second fixture project"]);
}

#[test]
fn the_agent_list_ranks_the_display_name_and_description() {
    let mut display = described_agent("build", "");
    display.display_name = "Builder".to_owned();
    let catalog = catalog_with(
        vec![],
        vec![],
        vec![
            display,
            described_agent("plan", "Reads the code before answering"),
        ],
        config(None, None),
    );
    let by_display: Vec<&str> = ranked_agents(&catalog, "builder")
        .iter()
        .map(|agent| agent.id.as_str())
        .collect();
    assert_eq!(by_display, ["build"]);
    let by_description: Vec<&str> = ranked_agents(&catalog, "reads")
        .iter()
        .map(|agent| agent.id.as_str())
        .collect();
    assert_eq!(by_description, ["plan"]);
}
