//! The settings a local pick wrote, kept beside the document until the server confirms them.
//!
//! React applies every settings read as the server document overlaid with the writes still in
//! flight and the changes still waiting for their debounce (`persistence.ts:441-466`,
//! `getSettingsWriteOverlay` and `reconcileSettingsRead`). Without that, a PUT answer or a
//! refetch that raced a later pick would put the older value back. A field the read already
//! holds exactly is dropped from the overlay, so a confirmed write stops pinning that key.
//!
//! `projects` is an array the server keeps whole; the overlay carries the picked array, so it
//! is merged by project id and only the fields this app patches
//! (`lastOpenedAt`, `sidebarCollapsed`) are applied over the server's objects.

use oc_client::settings::Settings;
use oc_client::{Project, ProjectPatch, SettingsPatch};

/// The read document with the unconfirmed local picks applied, and `overlay` cut down to the
/// fields that read did not confirm.
pub(crate) fn reconcile(server: &Settings, overlay: &mut SettingsPatch) -> Settings {
    let mut settings = server.clone();
    overlay_keys(server, &mut settings, overlay);
    settings.projects = reconcile_projects(server, overlay);
    settings
}

/// Applies every single-value key and drops the ones this read already holds.
fn overlay_keys(server: &Settings, settings: &mut Settings, overlay: &mut SettingsPatch) {
    macro_rules! keys {
        (optional: $($field:ident),+ $(,)?) => {
            $(
                if let Some(value) = overlay.$field.clone() {
                    if server.$field.as_ref() == Some(&value) {
                        overlay.$field = None;
                    }
                    settings.$field = Some(value);
                }
            )+
        };
        (plain: $($field:ident),+ $(,)?) => {
            $(
                if let Some(value) = overlay.$field.clone() {
                    if server.$field == value {
                        overlay.$field = None;
                    }
                    settings.$field = value;
                }
            )+
        };
    }
    keys!(optional: active_project_id, last_selected_model, default_model, default_variant, default_agent);
    keys!(
        plain: favorite_models,
        recent_models,
        hidden_models,
        favorite_agents,
        recent_agents,
        collapsed_model_providers,
        provider_order,
        last_used_efforts,
        enter_to_send,
        enter_to_send_configured,
        sidebar_project_sort_order,
        sidebar_keep_work_in_place,
        sidebar_show_chats_section,
        sidebar_show_work_section,
        sidebar_show_pinned_section,
        sidebar_sections,
        sidebar_hidden_sections,
        session_work_enabled,
    );
}

/// The merged `projects` array, and the overlay cut down to the projects the read does not
/// already match.
fn reconcile_projects(server: &Settings, overlay: &mut SettingsPatch) -> Vec<Project> {
    let merged = merge_projects(
        &server.projects,
        overlay.projects.as_deref().unwrap_or_default(),
    );
    let unsettled: Vec<Project> = overlay
        .projects
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|project| {
            !server
                .projects
                .iter()
                .any(|stored| same_project(stored, project))
        })
        .cloned()
        .collect();
    overlay.projects = (!unsettled.is_empty()).then_some(unsettled);
    merged
}

/// The server's projects with the overlay's patched fields applied by id; a project the
/// server does not hold yet is appended as the overlay states it.
fn merge_projects(server: &[Project], overlay: &[Project]) -> Vec<Project> {
    let mut merged = server.to_vec();
    for project in overlay {
        let Some(index) = merged.iter().position(|stored| stored.id() == project.id()) else {
            merged.push(project.clone());
            continue;
        };
        merged[index] = merge_project(&merged[index], project);
    }
    merged
}

/// Applies the two fields this app patches; the server's `color`, `icon` and any other field
/// stay byte for byte.
fn merge_project(server: &Project, overlay: &Project) -> Project {
    let patch = ProjectPatch {
        last_opened_at: changed(server.last_opened_at(), overlay.last_opened_at()),
        sidebar_collapsed: changed(server.sidebar_collapsed(), overlay.sidebar_collapsed()),
        ..ProjectPatch::default()
    };
    server.with_patch(&patch)
}

fn changed<T: Copy + PartialEq>(server: Option<T>, overlay: Option<T>) -> Option<T> {
    overlay.filter(|value| server != Some(*value))
}

/// Whether the read already holds the fields the overlay could patch for this project.
fn same_project(server: &Project, overlay: &Project) -> bool {
    server.id() == overlay.id()
        && server.last_opened_at() == overlay.last_opened_at()
        && server.sidebar_collapsed() == overlay.sidebar_collapsed()
}

#[cfg(test)]
#[path = "settings_overlay_test.rs"]
mod settings_overlay_test;
