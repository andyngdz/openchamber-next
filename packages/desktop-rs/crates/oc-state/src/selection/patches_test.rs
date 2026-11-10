//! Every picker rule and the settings patch it produces.

use serde_json::{Value, json};

use super::super::value::ModelKey;
use super::{
    activate_project, add_recent_agent, add_recent_model, merge_patch,
    remember_last_selected_model, remember_last_used_effort, toggle_favorite_agent,
    toggle_favorite_model,
};
use crate::fixture_test::{api_settings, settings};
use oc_client::SettingsPatch;

fn key(provider_id: &str, id: &str) -> ModelKey {
    ModelKey::new(provider_id, id)
}

#[test]
fn recent_models_dedupe_prepend_and_keep_five() {
    let mut settings = api_settings();
    let patch = add_recent_model(&mut settings, &key("perf", "stream-300cps"));
    let recent = patch.recent_models.expect("recent patch");
    assert_eq!(recent[0].provider_id, "perf");
    assert_eq!(recent[0].model_id, "stream-300cps");
    assert_eq!(recent.len(), 2);

    for index in 0..6 {
        add_recent_model(&mut settings, &key("p", &format!("model-{index}")));
    }
    assert_eq!(settings.recent_models.len(), 5);
    assert_eq!(settings.recent_models[0].model_id, "model-5");
}

#[test]
fn last_selected_model_writes_only_when_it_changes() {
    let mut settings = api_settings();
    let before = settings.last_selected_model.clone();
    let unchanged = remember_last_selected_model(&mut settings, &key("perf", "stream-1200cps"));
    assert!(unchanged.is_empty());

    let patch = remember_last_selected_model(&mut settings, &key("perf", "stream-300cps"));
    assert_eq!(
        patch.last_selected_model.as_deref(),
        Some("perf/stream-300cps")
    );
    assert_ne!(settings.last_selected_model, before);
}

#[test]
fn a_recent_agent_already_listed_is_left_alone_and_a_new_one_is_prepended() {
    let mut settings = api_settings();
    assert!(add_recent_agent(&mut settings, "capture-primary").is_empty());

    let patch = add_recent_agent(&mut settings, "plan");
    let recent = patch.recent_agents.expect("recent agents");
    assert_eq!(recent[0], "plan");
    assert_eq!(recent.len(), 3);

    for index in 0..6 {
        add_recent_agent(&mut settings, &format!("agent-{index}"));
    }
    assert_eq!(settings.recent_agents.len(), 5);
    assert_eq!(settings.recent_agents[0], "agent-5");
}

#[test]
fn a_favorite_model_toggles_and_a_new_one_is_prepended() {
    let mut settings = api_settings();
    let existing = key("perf", "stream-600cps");
    let patch = toggle_favorite_model(&mut settings, &existing);
    let favorites = patch.favorite_models.expect("favorites");
    assert_eq!(favorites.len(), 1);
    assert_eq!(favorites[0].model_id, "alt-thinking");

    let added = key("opencode", "big-pickle");
    let patch = toggle_favorite_model(&mut settings, &added);
    let favorites = patch.favorite_models.expect("favorites");
    assert_eq!(favorites[0].model_id, "big-pickle");
    assert_eq!(favorites.len(), 2);
}

#[test]
fn a_favorite_agent_toggles_and_a_new_one_is_appended() {
    let mut settings = api_settings();
    let patch = toggle_favorite_agent(&mut settings, "capture-primary");
    assert!(patch.favorite_agents.expect("favorites").is_empty());

    let patch = toggle_favorite_agent(&mut settings, "plan");
    let favorites = patch.favorite_agents.expect("favorites");
    assert_eq!(favorites, ["plan"]);
    assert!(toggle_favorite_agent(&mut settings, "  ").is_empty());
}

#[test]
fn last_used_efforts_writes_a_named_level_or_null_for_default() {
    let mut settings = api_settings();
    let named = remember_last_used_effort(&mut settings, &key("p", "one"), Some("high"));
    let efforts = named.last_used_efforts.expect("efforts");
    assert_eq!(efforts.get("p/one"), Some(&Some("high".to_owned())));

    let default = remember_last_used_effort(&mut settings, &key("p", "one"), None);
    let efforts = default.last_used_efforts.expect("efforts");
    assert_eq!(efforts.get("p/one"), Some(&None));

    assert!(remember_last_used_effort(&mut settings, &key("p", "one"), None).is_empty());
}

#[test]
fn last_used_efforts_keeps_the_newest_write_and_the_512_cap() {
    let mut efforts = serde_json::Map::new();
    for index in 0..512 {
        efforts.insert(format!("p/m{index:03}"), json!("low"));
    }
    let mut settings = settings(json!({"lastUsedEfforts": Value::Object(efforts)}));
    let patch = remember_last_used_effort(&mut settings, &key("p", "new"), Some("high"));
    let efforts = patch.last_used_efforts.expect("efforts");
    assert_eq!(efforts.len(), 512);
    assert!(efforts.get("p/new").is_some());
    assert!(
        efforts.get("p/m000").is_none(),
        "the oldest entry is dropped"
    );
    let keys: Vec<&str> = efforts.iter().map(|(key, _)| key).collect();
    assert_eq!(keys.first(), Some(&"p/m001"));
    assert_eq!(keys.last(), Some(&"p/new"));
}

#[test]
fn a_project_pick_writes_active_project_and_last_opened_at() {
    let mut settings = api_settings();
    let second = settings
        .projects
        .iter()
        .find(|project| {
            project.id() == Some("path_L3RtcC9vYy1kZXNrdG9wLXJzLWNhcHR1cmUvcHJvamVjdC10d28")
        })
        .expect("second project")
        .clone();
    let patch = activate_project(&mut settings, second.id().expect("id"), 42).expect("patch");
    assert_eq!(
        patch.active_project_id.as_deref(),
        Some(second.id().expect("id"))
    );
    let projects = patch.projects.expect("projects");
    let object: Value = serde_json::to_value(&projects[1]).expect("json");
    assert_eq!(object["lastOpenedAt"], 42);

    assert!(
        activate_project(&mut settings, second.id().expect("id"), 99).is_none(),
        "the active project is not stamped again"
    );
    assert_eq!(settings.active_project_id.as_deref(), second.id());
}

#[test]
fn a_merge_keeps_earlier_keys_and_takes_later_values() {
    let mut target = SettingsPatch {
        last_selected_model: Some("p/one".to_owned()),
        ..SettingsPatch::default()
    };
    merge_patch(
        &mut target,
        SettingsPatch {
            last_selected_model: Some("p/two".to_owned()),
            recent_agents: Some(vec!["plan".to_owned()]),
            ..SettingsPatch::default()
        },
    );
    assert_eq!(target.last_selected_model.as_deref(), Some("p/two"));
    assert_eq!(target.recent_agents.expect("agents"), ["plan"]);
}
