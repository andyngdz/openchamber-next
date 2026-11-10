//! The derived sections from the captured settings, home info and global pages.

use std::collections::BTreeMap;

use serde_json::json;

use super::sections;
use crate::fixture_test::{api_home, api_sessions, api_settings, home, settings, sidebar_session};
use crate::load::session_entry;
use crate::session_list::SessionList;
use crate::sidebar::model::{MoreKey, ProjectGroup, SidebarSectionModel};

fn populated() -> (
    oc_client::Settings,
    oc_client::filesystem::Home,
    SessionList,
) {
    let mut list = SessionList::default();
    for page in ["session-list-global-page-1", "session-list-global-page-2"] {
        for session in api_sessions(page) {
            list.insert(session_entry(&session));
        }
    }
    (api_settings(), api_home(), list)
}

fn project_section(sections: &[SidebarSectionModel]) -> &[ProjectGroup] {
    sections
        .iter()
        .find_map(|section| match section {
            SidebarSectionModel::Projects { groups } => Some(groups.as_slice()),
            _ => None,
        })
        .expect("a Projects section")
}

#[test]
fn the_captured_sidebar_has_chats_pinned_work_and_two_projects() {
    let (settings, home, list) = populated();
    let sections = sections(&settings, &home, &list, &BTreeMap::new());

    let kinds: Vec<&str> = sections
        .iter()
        .map(|section| match section {
            SidebarSectionModel::Chats { .. } => "chats",
            SidebarSectionModel::Pinned { .. } => "pinned",
            SidebarSectionModel::InWork { .. } => "work",
            SidebarSectionModel::Projects { .. } => "projects",
        })
        .collect();
    assert_eq!(kinds, ["chats", "pinned", "work", "projects"]);

    let SidebarSectionModel::Chats { rows, .. } = &sections[0] else {
        panic!("chats");
    };
    let SidebarSectionModel::Pinned { rows: pinned, .. } = &sections[1] else {
        panic!("pinned");
    };
    let SidebarSectionModel::InWork { rows: work, .. } = &sections[2] else {
        panic!("work");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(pinned.len(), 1);
    assert!(work.is_empty());
    assert_eq!(project_section(&sections).len(), 2);
}

#[test]
fn the_fixture_projects_keep_their_collapse_state() {
    let (settings, home, list) = populated();
    let sections = sections(&settings, &home, &list, &BTreeMap::new());
    let groups = project_section(&sections);

    assert_eq!(groups[0].project.label, "Fixture project");
    assert!(!groups[0].project.collapsed);
    assert_eq!(groups[1].project.label, "Second fixture project");
    assert!(groups[1].project.collapsed);
}

#[test]
fn a_project_shows_seven_rows_and_seven_more_per_click() {
    let settings = settings(json!({"projects": [{"id": "p", "path": "/p/app"}]}));
    let home = home(json!({"home": "/home/u"}));
    let mut list = SessionList::default();
    for index in 0..10 {
        list.insert(session_entry(&sidebar_session(
            &format!("ses_{index}"),
            "/p/app",
            index,
        )));
    }

    let first = sections(&settings, &home, &list, &BTreeMap::new());
    let group = &project_section(&first)[0];
    assert_eq!(group.shown, 7);
    assert_eq!(group.rows.len(), 10);

    let mut more = BTreeMap::new();
    more.insert(MoreKey::Project("p".to_owned()), 1);
    let expanded = sections(&settings, &home, &list, &more);
    assert_eq!(project_section(&expanded)[0].shown, 10);
}

#[test]
fn chats_shows_three_then_seven_more() {
    let settings = settings(json!({"projects": []}));
    let home = home(json!({"home": "/home/u"}));
    let mut list = SessionList::default();
    for index in 0..10 {
        list.insert(session_entry(&sidebar_session(
            &format!("ses_{index}"),
            "/home/u/.config/openchamber/chats/2026-01-02/s",
            index,
        )));
    }

    let first = sections(&settings, &home, &list, &BTreeMap::new());
    let SidebarSectionModel::Chats { rows, shown } = &first[0] else {
        panic!("chats");
    };
    assert_eq!((rows.len(), *shown), (10, 3));

    let mut more = BTreeMap::new();
    more.insert(MoreKey::Chats, 1);
    let expanded = sections(&settings, &home, &list, &more);
    let SidebarSectionModel::Chats { shown, .. } = &expanded[0] else {
        panic!("chats");
    };
    assert_eq!(*shown, 10);
}
