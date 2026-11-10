use super::{Settings, SidebarSectionKind, SidebarSortOrder};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn decode(name: &str) -> Settings {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

#[test]
fn the_captured_settings_decode_into_the_typed_keys() {
    let settings = decode("settings.json");

    assert!(settings.active_project_id.is_some());
    assert_eq!(settings.projects.len(), 2);
    assert_eq!(
        settings.default_model.as_deref(),
        Some("perf/stream-600cps")
    );
    assert_eq!(settings.default_variant.as_deref(), Some("high"));
    assert_eq!(settings.default_agent.as_deref(), Some("capture-primary"));
    assert_eq!(
        settings.last_selected_model.as_deref(),
        Some("perf/stream-1200cps")
    );
    assert!(settings.enter_to_send);
    assert!(settings.enter_to_send_configured);
    assert_eq!(settings.favorite_models[0].provider_id, "perf".to_owned());
    assert_eq!(settings.favorite_models[0].model_id, "stream-600cps");
    assert_eq!(settings.hidden_models.len(), 1);
    assert_eq!(
        settings.recent_agents,
        ["capture-secondary", "capture-primary"]
    );
    assert_eq!(
        settings.last_used_efforts.get("perf/stream-600cps"),
        Some(&Some("low".to_owned()))
    );
    assert_eq!(
        settings.last_used_efforts.get("perf-second/alt-thinking"),
        Some(&None)
    );
    let effort_keys: Vec<&str> = settings
        .last_used_efforts
        .iter()
        .map(|(key, _)| key)
        .collect();
    assert_eq!(
        effort_keys,
        ["perf/stream-600cps", "perf-second/alt-thinking"]
    );
    assert_eq!(settings.sidebar_project_sort_order, SidebarSortOrder::AZ);
    assert_eq!(settings.sidebar_sections.len(), 4);
    assert_eq!(
        settings.sidebar_sections[0].kind,
        SidebarSectionKind::Builtin
    );
    assert_eq!(settings.sidebar_sections[0].id, "chats");
    assert!(settings.sidebar_show_chats_section);
    assert!(settings.sidebar_show_work_section);
    assert!(settings.sidebar_show_pinned_section);
    assert!(settings.session_work_enabled);
}

#[test]
fn a_settings_document_without_sidebar_keys_takes_the_captured_defaults() {
    let settings: Settings = serde_json::from_str(r#"{"projects":[]}"#).expect("decode");

    assert!(!settings.enter_to_send);
    assert!(!settings.enter_to_send_configured);
    assert_eq!(
        settings.sidebar_project_sort_order,
        SidebarSortOrder::Manual
    );
    assert!(settings.sidebar_show_chats_section);
    assert!(settings.sidebar_show_work_section);
    assert!(settings.sidebar_show_pinned_section);
    assert!(settings.session_work_enabled);
    assert!(settings.sidebar_sections.is_empty());
    assert!(settings.projects.is_empty());
}

#[test]
fn an_unknown_project_sort_order_does_not_fail_the_document() {
    let settings: Settings =
        serde_json::from_str(r#"{"sidebarProjectSortOrder":"by-color","projects":[]}"#)
            .expect("decode");

    assert_eq!(settings.sidebar_project_sort_order, SidebarSortOrder::Other);
}

#[test]
fn last_used_efforts_keep_document_order_through_decode_and_encode() {
    let settings: Settings = serde_json::from_str(
        r#"{"lastUsedEfforts":{"z/model":"high","a/model":null,"m/model":"low"}}"#,
    )
    .expect("decode");
    let keys: Vec<&str> = settings
        .last_used_efforts
        .iter()
        .map(|(key, _)| key)
        .collect();
    assert_eq!(keys, ["z/model", "a/model", "m/model"]);
    let encoded = serde_json::to_string(&settings.last_used_efforts).expect("encode");
    assert_eq!(
        encoded,
        r#"{"z/model":"high","a/model":null,"m/model":"low"}"#
    );
}
