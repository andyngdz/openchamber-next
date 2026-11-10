use oc_client::filesystem::Home;

use super::{chat_folder_for, chats_root, new_chat_folder};

fn home(home: &str, chats_root: &str) -> Home {
    Home {
        home: home.to_owned(),
        chats_root: chats_root.to_owned(),
        canonical_chats_root: None,
        canonical_legacy_chats_root: None,
    }
}

#[test]
fn the_configured_chats_root_wins() {
    let info = home("/home/user", "/data/chats/");

    assert_eq!(chats_root(&info).as_deref(), Some("/data/chats"));
}

#[test]
fn the_legacy_root_follows_the_home_when_unconfigured() {
    let info = home("/home/user/", "");

    assert_eq!(
        chats_root(&info).as_deref(),
        Some("/home/user/.config/openchamber/chats")
    );
}

#[test]
fn no_home_and_no_chats_root_means_no_folder() {
    assert_eq!(chats_root(&home("", "")), None);
    assert_eq!(new_chat_folder(&home("", "")), None);
}

#[test]
fn a_known_uuid_builds_the_path_the_app_opens() {
    let folder = chat_folder_for("/chats/", "00000000-0000-0000-0000-000000000000");

    assert!(folder.starts_with("/chats/"), "{folder}");
    assert!(
        folder.ends_with("/session-00000000-0000-0000-0000-000000000000"),
        "{folder}"
    );
}

#[test]
fn a_new_folder_carries_the_day_and_a_uuid_v4() {
    let Some(folder) = new_chat_folder(&home("/home/user", "/chats")) else {
        panic!("a folder expected");
    };
    let rest = folder
        .strip_prefix("/chats/")
        .unwrap_or_else(|| panic!("unexpected folder {folder}"));
    let (day, name) = rest
        .split_once('/')
        .unwrap_or_else(|| panic!("unexpected {rest}"));
    assert_eq!(day.len(), 10, "{day}");
    assert!(day.split('-').count() == 3, "{day}");
    let id = name
        .strip_prefix("session-")
        .unwrap_or_else(|| panic!("unexpected {name}"));
    assert_eq!(id.len(), 36, "{id}");
    assert!(id.chars().filter(|c| *c == '-').count() == 4, "{id}");
}
