//! Picking the draft's target.

use serde_json::Value;

use crate::fixture_test::api_settings;
use crate::selection::{DraftTarget, ModelKey, SelectionAction};
use crate::sidebar_state::SidebarAction;
use crate::state::State;

const SECOND: &str = "path_L3RtcC9vYy1kZXNrdG9wLXJzLWNhcHR1cmUvcHJvamVjdC10d28";
const FIRST: &str = "path_L3RtcC9vYy1kZXNrdG9wLXJzLWNhcHR1cmUvcHJvamVjdA";

fn loaded() -> State {
    let mut state = State::new("/home/u");
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(api_settings())));
    state
}

#[test]
fn picking_a_project_targets_it_makes_it_active_and_stamps_last_opened_at() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickProject {
        id: SECOND.to_owned(),
        now: 123,
    });

    match state.draft().target.clone() {
        DraftTarget::Project { id, directory } => {
            assert_eq!(id, SECOND);
            assert_eq!(directory, "/tmp/oc-desktop-rs-capture/project-two");
        }
        DraftTarget::Chat => panic!("expected a project target"),
    }

    let patch = state.take_settings_patch().expect("patch");
    assert_eq!(patch.active_project_id.as_deref(), Some(SECOND));
    let projects = patch.projects.expect("projects");
    let object: Value = serde_json::to_value(&projects[1]).expect("json");
    assert_eq!(object["lastOpenedAt"], 123);
    assert_eq!(object["color"], Value::Null);

    let model = state
        .current_selection()
        .and_then(|selection| selection.model())
        .expect("settings default model");
    assert_eq!(model, &ModelKey::new("perf", "stream-600cps"));
}

#[test]
fn picking_the_project_already_active_changes_no_settings() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickProject {
        id: FIRST.to_owned(),
        now: 42,
    });
    assert!(state.take_settings_patch().is_none());
    assert_eq!(state.draft().target.project_id(), Some(FIRST));
}

#[test]
fn picking_chat_targets_chat_and_leaves_the_active_project_alone() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickChat);
    assert_eq!(state.draft().target, DraftTarget::Chat);
    assert!(state.take_settings_patch().is_none());
}

#[test]
fn an_unknown_project_is_ignored() {
    let mut state = loaded();
    state.apply_selection(SelectionAction::PickProject {
        id: "nope".to_owned(),
        now: 1,
    });
    assert_eq!(state.draft().target, DraftTarget::Chat);
    assert!(state.take_settings_patch().is_none());
}
