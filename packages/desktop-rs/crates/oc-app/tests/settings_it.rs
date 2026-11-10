//! What the project's stored settings keep through a write from this app: the fields only
//! React names, untouched while a sidebar action writes the project back.
//!
//! Runs only with `OC_DESKTOP_IT=1`:
//! `OC_DESKTOP_IT=1 cargo test -p oc-app --test settings_it -- --nocapture`.

use std::path::Path;

use oc_client::{Client, Settings, SettingsPatch};
use oc_state::{SidebarAction, State};
use serde_json::{Value, json};

mod common;
mod support;

use common::{Env, enabled, fail};
use support::{runtime, start, stop};

/// The fields `common::Env` seeds on the project and this app only round-trips.
const PROJECT_COLOR: &str = "#4F46E5";
const PROJECT_ICON: &str = "rocket";
const PROJECT_ICON_BACKGROUND: &str = "#e0e7ff";

/// The stored id and object of the project at `path`, as the settings document keeps it.
fn project_of(settings: &Settings, path: &Path) -> (String, Value) {
    let path = std::fs::canonicalize(path).unwrap_or_else(|error| fail("project path", error));
    let path = path.to_string_lossy();
    let project = settings
        .projects
        .iter()
        .find(|project| project.path() == Some(path.as_ref()))
        .unwrap_or_else(|| fail("settings project", format!("no project at {path}")));
    let id = project
        .id()
        .unwrap_or_else(|| fail("settings project", "no id"))
        .to_owned();
    let value = serde_json::to_value(project).unwrap_or_else(|error| fail("project JSON", error));
    (id, value)
}

fn field_bytes(project: &Value, field: &str) -> String {
    project
        .get(field)
        .unwrap_or_else(|| fail("project field", format!("{field} is missing")))
        .to_string()
}

/// The settings write a project collapse queues, through the state's own patch path.
fn collapse_patch(settings: &Settings, directory: &str, id: &str) -> SettingsPatch {
    let mut state = State::new(directory.to_owned());
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings.clone())));
    state.apply_sidebar(SidebarAction::ToggleProjectCollapsed(id.to_owned()));
    state
        .take_settings_patch()
        .unwrap_or_else(|| fail("settings patch", "the collapse queued no write"))
}

#[test]
fn a_sidebar_write_keeps_the_project_fields_this_app_does_not_model() {
    if !enabled() {
        return;
    }
    let env = Env::new();
    let sidecar = start(&env);
    let client = Client::new(sidecar.port()).unwrap_or_else(|error| fail("client", error));
    let project = env.project.to_string_lossy().into_owned();
    let second = env.root.join("second-project");
    runtime().block_on(async {
        client
            .check_auth()
            .await
            .unwrap_or_else(|error| fail("auth", error));
        let before = client
            .settings()
            .await
            .unwrap_or_else(|error| fail("settings", error));
        let (id, before_project) = project_of(&before, &env.project);
        for (field, expected) in [
            ("color", PROJECT_COLOR),
            ("icon", PROJECT_ICON),
            ("iconBackground", PROJECT_ICON_BACKGROUND),
        ] {
            assert_eq!(
                field_bytes(&before_project, field),
                json!(expected).to_string(),
                "{field} is not seeded"
            );
        }

        let patch = collapse_patch(&before, &project, &id);
        let written = client
            .update_settings(&patch)
            .await
            .unwrap_or_else(|error| fail("update settings", error));
        let after = client
            .settings()
            .await
            .unwrap_or_else(|error| fail("settings after write", error));
        let mut expected = before_project.clone();
        expected["sidebarCollapsed"] = json!(true);
        assert_eq!(
            project_of(&written, &env.project).1,
            expected,
            "the response changed a field the write did not name"
        );
        assert_eq!(
            project_of(&after, &env.project).1,
            expected,
            "the stored project changed a field the write did not name"
        );
        assert_eq!(
            project_of(&after, &second).1,
            project_of(&before, &second).1,
            "the write changed the second project"
        );
    });
    stop(sidecar);
}
