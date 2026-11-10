//! Section order, the show toggles and the hidden list.

use serde_json::json;

use super::{BuiltinSection, visible};
use crate::fixture_test::settings;

#[test]
fn an_empty_stored_list_becomes_the_default_four() {
    let settings = settings(json!({"projects": []}));

    assert_eq!(
        visible(&settings),
        [
            BuiltinSection::Chats,
            BuiltinSection::Pinned,
            BuiltinSection::Work,
            BuiltinSection::Projects,
        ]
    );
}

#[test]
fn the_stored_order_wins_and_custom_sections_are_left_out() {
    let settings = settings(json!({
        "projects": [],
        "sidebarSections": [
            {"kind": "builtin", "id": "projects"},
            {"kind": "builtin", "id": "chats"},
            {"kind": "custom", "id": "section:mine", "name": "Mine"},
            {"kind": "builtin", "id": "projects"},
        ],
    }));

    assert_eq!(
        visible(&settings),
        [
            BuiltinSection::Projects,
            BuiltinSection::Chats,
            BuiltinSection::Pinned,
            BuiltinSection::Work,
        ]
    );
}

#[test]
fn the_show_toggles_hide_a_builtin_and_work_needs_its_own_switch() {
    let settings = settings(json!({
        "projects": [],
        "sidebarShowChatsSection": false,
        "sidebarShowPinnedSection": false,
        "sidebarShowWorkSection": true,
        "sessionWorkEnabled": false,
    }));

    assert_eq!(visible(&settings), [BuiltinSection::Projects]);
}

#[test]
fn the_hidden_list_removes_a_builtin_too() {
    let settings = settings(json!({
        "projects": [],
        "sidebarHiddenSections": ["chats", "work"],
    }));

    assert_eq!(
        visible(&settings),
        [BuiltinSection::Pinned, BuiltinSection::Projects]
    );
}
