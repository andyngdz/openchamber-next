use serde_json::json;

use super::SettingsPatch;
use crate::settings::{LastUsedEfforts, SettingsModelRef, SidebarSortOrder};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn decode_patch(name: &str) -> SettingsPatch {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

#[test]
fn the_captured_partial_put_decodes_to_only_the_keys_it_names() {
    let patch = decode_patch("settings-update.request.json");

    assert!(patch.active_project_id.is_some());
    assert_eq!(patch.recent_models.as_ref().map(Vec::len), Some(2));
    assert_eq!(
        patch.last_selected_model.as_deref(),
        Some("perf/stream-600cps")
    );
    assert_eq!(patch.last_used_efforts.as_ref().map(|e| e.len()), Some(2));
    let efforts = patch.last_used_efforts.as_ref().expect("efforts");
    let keys: Vec<&str> = efforts.iter().map(|(key, _)| key).collect();
    assert_eq!(keys, ["perf/stream-600cps", "perf-second/alt-thinking"]);
    assert_eq!(patch.projects, None);
    assert_eq!(patch.favorite_models, None);
}

#[test]
fn a_patch_serializes_only_the_fields_it_sets() {
    let patch = SettingsPatch {
        sidebar_project_sort_order: Some(SidebarSortOrder::Recent),
        recent_models: Some(vec![SettingsModelRef {
            provider_id: "perf".to_owned(),
            model_id: "stream-600cps".to_owned(),
        }]),
        ..SettingsPatch::default()
    };

    assert_eq!(
        serde_json::to_value(&patch).expect("serialize"),
        json!({
            "sidebarProjectSortOrder": "recent",
            "recentModels": [{"providerID": "perf", "modelID": "stream-600cps"}],
        })
    );
}

#[test]
fn an_empty_patch_serializes_to_an_empty_object() {
    assert!(SettingsPatch::default().is_empty());
    assert_eq!(
        serde_json::to_value(SettingsPatch::default()).expect("serialize"),
        json!({})
    );
}

#[test]
fn last_used_efforts_serialize_in_their_stored_order() {
    let efforts: LastUsedEfforts =
        serde_json::from_str(r#"{"z/model":"high","a/model":null}"#).expect("efforts");
    let patch = SettingsPatch {
        last_used_efforts: Some(efforts),
        ..SettingsPatch::default()
    };
    assert_eq!(
        serde_json::to_string(&patch).expect("serialize"),
        r#"{"lastUsedEfforts":{"z/model":"high","a/model":null}}"#
    );
}
