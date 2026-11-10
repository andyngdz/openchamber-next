//! The four chat roots, project membership by path, and path normalization.

use serde_json::json;

use super::{
    ChatRoots, display_label, format_directory_name, normalize_path, project_index, split,
};
use crate::fixture_test::{home, sidebar_session};
use crate::load::session_entry;
use crate::session_list::SessionList;

fn list(directories: &[(&str, &str)]) -> SessionList {
    let mut list = SessionList::default();
    for (id, directory) in directories {
        list.insert(session_entry(&sidebar_session(id, directory, 1)));
    }
    list
}

fn roots() -> ChatRoots {
    ChatRoots::from_home(&home(json!({
        "home": "/home/u",
        "chatsRoot": "/home/u/.oc/chats",
        "canonicalChatsRoot": "/home/u/canon",
        "canonicalLegacyChatsRoot": "/home/u/legacy-canon",
    })))
    .expect("roots")
}

#[test]
fn every_chat_root_classifies_its_sessions() {
    let sessions = list(&[
        ("ses_configured", "/home/u/.oc/chats/2026-01-02/s"),
        ("ses_canonical", "/home/u/canon/s"),
        ("ses_legacy", "/home/u/.config/openchamber/chats/s"),
        ("ses_legacy_canonical", "/home/u/legacy-canon/s"),
        ("ses_outside", "/home/u/.oc/chats-other/s"),
    ]);

    let split = split(&sessions, &[], &roots());
    let ids: Vec<&str> = split
        .chats
        .iter()
        .map(|session| session.id.as_str())
        .collect();

    assert_eq!(split.chats.len(), 4);
    for id in [
        "ses_configured",
        "ses_canonical",
        "ses_legacy",
        "ses_legacy_canonical",
    ] {
        assert!(ids.contains(&id), "{id} missing from {ids:?}");
    }
    assert!(!ids.contains(&"ses_outside"));
}

#[test]
fn a_session_belongs_to_the_closest_project() {
    let projects: Vec<oc_client::Project> = serde_json::from_value(json!([
        {"id": "outer", "path": "/p/app"},
        {"id": "inner", "path": "/p/app/nested"},
    ]))
    .expect("projects");

    assert_eq!(project_index("/p/app/x", &projects), Some(0));
    assert_eq!(project_index("/p/app/nested/x", &projects), Some(1));
    assert_eq!(project_index("/p/application/x", &projects), None);
    assert_eq!(project_index("/elsewhere", &projects), None);
}

#[test]
fn archived_and_btw_sessions_never_join_a_bucket() {
    let archived = crate::fixture_test::sidebar_from(json!({
        "id": "ses_archived",
        "location": {"directory": "/p/app"},
        "time": {"created": 1, "updated": 2, "archived": 3},
    }));
    let btw = crate::fixture_test::sidebar_from(json!({
        "id": "ses_btw",
        "location": {"directory": "/p/app"},
        "metadata": {"openchamber": {"kind": "btw", "originalSessionID": "ses_a"}},
    }));
    let plain = sidebar_session("ses_plain", "/p/app", 1);
    let mut sessions = SessionList::default();
    sessions.insert(session_entry(&archived));
    sessions.insert(session_entry(&btw));
    sessions.insert(session_entry(&plain));
    let projects: Vec<oc_client::Project> =
        serde_json::from_value(json!([{"id": "app", "path": "/p/app"}])).expect("projects");

    let split = split(&sessions, &projects, &roots());

    assert_eq!(split.projects[0].len(), 1);
    assert_eq!(split.projects[0][0].id.as_str(), "ses_plain");
}

#[test]
fn path_normalization_matches_the_react_helper() {
    assert_eq!(normalize_path(" /p/app/ "), Some("/p/app".to_owned()));
    assert_eq!(
        normalize_path("/p/app\\sub/"),
        Some("/p/app/sub".to_owned())
    );
    assert_eq!(normalize_path("c:/app"), Some("C:/app".to_owned()));
    assert_eq!(normalize_path("/"), Some("/".to_owned()));
    assert_eq!(normalize_path("   "), None);
}

#[test]
fn labels_fall_back_to_the_folder_name() {
    let with_label: oc_client::Project =
        serde_json::from_value(json!({"id": "a", "path": "/p/app", "label": " My app "}))
            .expect("project");
    let without: oc_client::Project =
        serde_json::from_value(json!({"id": "b", "path": "/p/app"})).expect("project");

    assert_eq!(display_label(&with_label, "/home/u"), "My app");
    assert_eq!(display_label(&without, "/home/u"), "app");
    assert_eq!(format_directory_name("/home/u", "/home/u"), "~");
    assert_eq!(format_directory_name("/", "/home/u"), "/");
}
