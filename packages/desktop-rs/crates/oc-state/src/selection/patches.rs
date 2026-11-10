//! The settings changes a pick makes (`useUIStore.ts:3028-3286`, `useProjectsStore.ts:844-849`).
//!
//! Each rule updates the local settings document and returns the partial `PUT` body that
//! carries the change. A later pick merges into the pending patch instead of replacing it.

use oc_client::SettingsPatch;
use oc_client::project::ProjectPatch;
use oc_client::settings::{Settings, SettingsModelRef};

use super::value::ModelKey;

/// Recent models and recent agents keep five entries.
pub(crate) const RECENT_LIMIT: usize = 5;
/// `lastUsedEfforts` keeps the newest 512 model keys.
pub(crate) const MAX_LAST_USED_EFFORTS: usize = 512;

/// Recent gets the model, deduplicated, at the front, at most five.
pub(crate) fn add_recent_model(settings: &mut Settings, key: &ModelKey) -> SettingsPatch {
    let reference = settings_ref(key);
    settings.recent_models.retain(|item| {
        item.provider_id != reference.provider_id || item.model_id != reference.model_id
    });
    settings.recent_models.insert(0, reference);
    settings.recent_models.truncate(RECENT_LIMIT);
    SettingsPatch {
        recent_models: Some(settings.recent_models.clone()),
        ..SettingsPatch::default()
    }
}

/// `lastSelectedModel` becomes `provider/id`, when it changes.
pub(crate) fn remember_last_selected_model(
    settings: &mut Settings,
    key: &ModelKey,
) -> SettingsPatch {
    let next = key.settings_key();
    if settings.last_selected_model.as_deref() == Some(next.as_str()) {
        return SettingsPatch::default();
    }
    settings.last_selected_model = Some(next.clone());
    SettingsPatch {
        last_selected_model: Some(next),
        ..SettingsPatch::default()
    }
}

/// A new agent is prepended to Recent; one already there leaves the list alone.
pub(crate) fn add_recent_agent(settings: &mut Settings, agent: &str) -> SettingsPatch {
    let name = agent.trim();
    if name.is_empty() || settings.recent_agents.iter().any(|entry| entry == name) {
        return SettingsPatch::default();
    }
    settings.recent_agents.insert(0, name.to_owned());
    settings.recent_agents.truncate(RECENT_LIMIT);
    SettingsPatch {
        recent_agents: Some(settings.recent_agents.clone()),
        ..SettingsPatch::default()
    }
}

/// A starred model toggles; a new one is prepended.
pub(crate) fn toggle_favorite_model(settings: &mut Settings, key: &ModelKey) -> SettingsPatch {
    let reference = settings_ref(key);
    let exists = settings.favorite_models.iter().any(|item| {
        item.provider_id == reference.provider_id && item.model_id == reference.model_id
    });
    if exists {
        settings.favorite_models.retain(|item| {
            item.provider_id != reference.provider_id || item.model_id != reference.model_id
        });
    } else {
        settings.favorite_models.insert(0, reference);
    }
    SettingsPatch {
        favorite_models: Some(settings.favorite_models.clone()),
        ..SettingsPatch::default()
    }
}

/// A starred agent toggles; a new one is appended.
pub(crate) fn toggle_favorite_agent(settings: &mut Settings, agent: &str) -> SettingsPatch {
    let name = agent.trim();
    if name.is_empty() {
        return SettingsPatch::default();
    }
    if settings.favorite_agents.iter().any(|entry| entry == name) {
        settings.favorite_agents.retain(|entry| entry != name);
    } else {
        settings.favorite_agents.push(name.to_owned());
    }
    SettingsPatch {
        favorite_agents: Some(settings.favorite_agents.clone()),
        ..SettingsPatch::default()
    }
}

/// Records the Thinking level sent or picked for a model; `None` is an explicit Default.
pub(crate) fn remember_last_used_effort(
    settings: &mut Settings,
    key: &ModelKey,
    variant: Option<&str>,
) -> SettingsPatch {
    let normalized = variant
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let memory_key = key.settings_key();
    if settings.last_used_efforts.get(&memory_key) == Some(&normalized) {
        return SettingsPatch::default();
    }
    settings
        .last_used_efforts
        .insert(memory_key.clone(), normalized);
    settings
        .last_used_efforts
        .keep_newest(MAX_LAST_USED_EFFORTS);
    SettingsPatch {
        last_used_efforts: Some(settings.last_used_efforts.clone()),
        ..SettingsPatch::default()
    }
}

/// Makes `id` the active project and stamps its `lastOpenedAt`, as `setActiveProject` does.
///
/// Picking the project already active changes nothing, not even `lastOpenedAt`.
pub(crate) fn activate_project(
    settings: &mut Settings,
    id: &str,
    now: u64,
) -> Option<SettingsPatch> {
    if settings.active_project_id.as_deref() == Some(id) {
        return None;
    }
    let index = settings
        .projects
        .iter()
        .position(|project| project.id() == Some(id))?;
    let patch = ProjectPatch {
        last_opened_at: Some(now),
        ..ProjectPatch::default()
    };
    settings.projects[index] = settings.projects[index].with_patch(&patch);
    settings.active_project_id = Some(id.to_owned());
    Some(SettingsPatch {
        active_project_id: Some(id.to_owned()),
        projects: Some(settings.projects.clone()),
        ..SettingsPatch::default()
    })
}

fn settings_ref(key: &ModelKey) -> SettingsModelRef {
    SettingsModelRef {
        provider_id: key.provider_id().to_owned(),
        model_id: key.id().to_owned(),
    }
}

/// Folds `next` into `target`, a later value winning.
pub(crate) fn merge_patch(target: &mut SettingsPatch, next: SettingsPatch) {
    keep(&mut target.active_project_id, next.active_project_id);
    keep(&mut target.favorite_models, next.favorite_models);
    keep(&mut target.recent_models, next.recent_models);
    keep(&mut target.hidden_models, next.hidden_models);
    keep(&mut target.favorite_agents, next.favorite_agents);
    keep(&mut target.recent_agents, next.recent_agents);
    keep(
        &mut target.collapsed_model_providers,
        next.collapsed_model_providers,
    );
    keep(&mut target.provider_order, next.provider_order);
    keep(&mut target.last_used_efforts, next.last_used_efforts);
    keep(&mut target.last_selected_model, next.last_selected_model);
    keep(&mut target.default_model, next.default_model);
    keep(&mut target.default_variant, next.default_variant);
    keep(&mut target.default_agent, next.default_agent);
    keep(&mut target.enter_to_send, next.enter_to_send);
    keep(
        &mut target.enter_to_send_configured,
        next.enter_to_send_configured,
    );
    keep(
        &mut target.sidebar_project_sort_order,
        next.sidebar_project_sort_order,
    );
    keep(
        &mut target.sidebar_keep_work_in_place,
        next.sidebar_keep_work_in_place,
    );
    keep(
        &mut target.sidebar_show_chats_section,
        next.sidebar_show_chats_section,
    );
    keep(
        &mut target.sidebar_show_work_section,
        next.sidebar_show_work_section,
    );
    keep(
        &mut target.sidebar_show_pinned_section,
        next.sidebar_show_pinned_section,
    );
    keep(&mut target.sidebar_sections, next.sidebar_sections);
    keep(
        &mut target.sidebar_hidden_sections,
        next.sidebar_hidden_sections,
    );
    keep(&mut target.session_work_enabled, next.session_work_enabled);
    keep(&mut target.projects, next.projects);
}

fn keep<T>(slot: &mut Option<T>, next: Option<T>) {
    if next.is_some() {
        *slot = next;
    }
}

#[cfg(test)]
#[path = "patches_test.rs"]
mod patches_test;
