use oc_client::SettingsPatch;
use oc_client::settings::Settings;
use serde_json::json;

use super::{abandon, reconcile};
use crate::{SidebarAction, State};

fn settings(value: serde_json::Value) -> Settings {
    serde_json::from_value(value).expect("settings")
}

fn patch(value: serde_json::Value) -> SettingsPatch {
    serde_json::from_value(value).expect("patch")
}

#[test]
fn a_read_keeps_a_later_pick_and_drops_the_confirmed_one() {
    let server = settings(json!({"favoriteAgents": ["a"]}));
    let mut overlay = patch(json!({
        "favoriteAgents": ["a"],
        "favoriteModels": [{"providerID": "p", "modelID": "m"}],
    }));

    let document = reconcile(&server, &mut overlay);

    assert_eq!(document.favorite_agents, ["a".to_owned()]);
    assert_eq!(document.favorite_models.len(), 1);
    assert!(
        overlay.favorite_agents.is_none(),
        "the read confirmed the starred agent"
    );
    assert!(overlay.favorite_models.is_some(), "the older pick stays");
}

#[test]
fn a_project_collapse_merges_by_id_and_keeps_the_server_fields() {
    let server = settings(json!({"projects": [{
        "id": "p", "path": "/p", "color": "#4F46E5", "icon": "rocket",
        "lastOpenedAt": 1000, "sidebarCollapsed": false,
    }]}));
    let mut overlay = patch(json!({"projects": [{
        "id": "p", "path": "/p", "lastOpenedAt": 1000, "sidebarCollapsed": true,
    }]}));

    let document = reconcile(&server, &mut overlay);

    assert_eq!(document.projects.len(), 1);
    assert_eq!(document.projects[0].sidebar_collapsed(), Some(true));
    assert_eq!(document.projects[0].last_opened_at(), Some(1000));
    let object = serde_json::to_value(&document.projects[0]).expect("json");
    assert_eq!(object["color"], "#4F46E5");
    assert_eq!(object["icon"], "rocket");
    assert!(overlay.projects.is_some(), "the collapse is not confirmed");
}

#[test]
fn a_confirmed_project_drops_out_of_the_overlay() {
    let server = settings(json!({"projects": [{
        "id": "p", "path": "/p", "lastOpenedAt": 1000, "sidebarCollapsed": true,
    }]}));
    let mut overlay = patch(json!({"projects": [{
        "id": "p", "path": "/p", "lastOpenedAt": 1000, "sidebarCollapsed": true,
    }]}));

    reconcile(&server, &mut overlay);

    assert!(overlay.projects.is_none());
}

#[test]
fn an_abandoned_write_clears_only_its_own_values() {
    let mut overlay = patch(json!({
        "defaultAgent": "build",
        "favoriteAgents": ["a"],
    }));
    let abandoned = patch(json!({
        "defaultAgent": "build",
        "favoriteAgents": ["a"],
    }));
    // A later pick changed the starred agents after this write was queued.
    overlay.favorite_agents = Some(vec!["a".to_owned(), "b".to_owned()]);

    abandon(&mut overlay, &abandoned);

    assert!(
        overlay.default_agent.is_none(),
        "the abandoned agent leaves"
    );
    assert_eq!(
        overlay.favorite_agents,
        Some(vec!["a".to_owned(), "b".to_owned()]),
        "a key a later pick changed stays"
    );
}

#[test]
fn a_held_write_stays_in_the_overlay_for_the_next_connection() {
    let mut state = State::new("/home/u");
    state.queue_settings_patch(patch(json!({"defaultAgent": "build"})));
    state.queue_settings_patch(patch(json!({"favoriteAgents": ["a"]})));

    // `after_change` takes the merged write; with no connection the link holds it, and the
    // whole overlay is what a new connection resends as one write.
    let held = state
        .take_settings_patch()
        .expect("the picks queued a write");
    let overlay = state
        .settings_overlay()
        .expect("the held write stays in the overlay");

    assert_eq!(overlay, held);
    assert_eq!(
        overlay.favorite_agents,
        Some(vec!["a".to_owned()]),
        "every key of the held write is resent"
    );
}

#[test]
fn abandoning_a_dropped_write_makes_the_next_read_authoritative() {
    let mut state = State::new("/home/u");
    state.queue_settings_patch(patch(json!({"defaultAgent": "build"})));
    let sent = state
        .take_settings_patch()
        .expect("the pick queued a write");

    // The live queue was full, so the write is abandoned.
    state.abandon_settings(&sent);

    // A later read carries the server's value; the abandoned pick must not mask it.
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings(json!({
        "defaultAgent": "plan",
        "projects": [],
    })))));

    assert_eq!(
        state.settings().expect("settings").default_agent.as_deref(),
        Some("plan"),
        "the abandoned key no longer masks the server"
    );
}
