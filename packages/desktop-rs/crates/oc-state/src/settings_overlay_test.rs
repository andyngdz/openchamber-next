use oc_client::SettingsPatch;
use oc_client::settings::Settings;
use serde_json::json;

use super::reconcile;

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
