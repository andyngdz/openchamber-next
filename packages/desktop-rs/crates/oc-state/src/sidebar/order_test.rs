//! Project order for every setting.

use serde_json::json;

use super::order_projects;
use crate::fixture_test::{home, sidebar_session};
use crate::load::session_entry;
use crate::session_list::SessionList;
use crate::sidebar::split::{ChatRoots, split};
use oc_client::Project;
use oc_client::settings::SidebarSortOrder;

fn projects() -> Vec<Project> {
    serde_json::from_value(json!([
        {"id": "a", "path": "/p/beta", "label": "Beta", "addedAt": 100, "lastOpenedAt": 10},
        {"id": "b", "path": "/p/alpha", "label": "alpha", "addedAt": 300, "lastOpenedAt": 30},
        {"id": "c", "path": "/p/gamma", "addedAt": 200, "lastOpenedAt": 20},
    ]))
    .expect("projects")
}

fn sessions() -> SessionList {
    let mut list = SessionList::default();
    list.insert(session_entry(&sidebar_session("ses_a", "/p/beta", 5)));
    list.insert(session_entry(&sidebar_session("ses_b", "/p/alpha", 50)));
    list
}

fn order(setting: SidebarSortOrder) -> Vec<usize> {
    let projects = projects();
    let roots = ChatRoots::from_home(&home(json!({"home": "/home/u"}))).expect("roots");
    let sessions = sessions();
    let split = split(&sessions, &projects, &roots);
    order_projects(&projects, setting, &split)
}

#[test]
fn manual_keeps_the_settings_order() {
    assert_eq!(order(SidebarSortOrder::Manual), [0, 1, 2]);
}

#[test]
fn an_unknown_order_keeps_the_settings_order_as_manual_does() {
    assert_eq!(order(SidebarSortOrder::Other), [0, 1, 2]);
}

#[test]
fn a_z_sorts_by_label_then_path_lowercased() {
    assert_eq!(order(SidebarSortOrder::AZ), [2, 1, 0]);
}

#[test]
fn z_a_reverses_that_order() {
    assert_eq!(order(SidebarSortOrder::ZA), [0, 1, 2]);
}

#[test]
fn date_added_is_newest_first() {
    assert_eq!(order(SidebarSortOrder::DateAdded), [1, 2, 0]);
}

#[test]
fn recent_reranks_by_the_newest_session_activity() {
    assert_eq!(order(SidebarSortOrder::Recent), [1, 0, 2]);
}
