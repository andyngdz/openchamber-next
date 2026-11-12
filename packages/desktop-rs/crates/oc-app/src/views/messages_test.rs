use std::path::Path;

use serde_json::Value;

use super::{META_ACTIONS, SCROLL_TO_BOTTOM, SCROLLER, USER_ACTIONS};
use crate::views::reference_test::names as reference_names;

/// Accessible names of a turn state's chat column, read from its reference.
fn turn_names(state: &str) -> Vec<String> {
    let path = Path::new(env!("OC_DESKTOP_FIXTURES"))
        .join("reference")
        .join("turn")
        .join(format!("{state}.json"));
    let text = std::fs::read_to_string(&path).expect("turn reference reads");
    let value: Value = serde_json::from_str(&text).expect("turn reference is JSON");
    value["regions"]["chatColumn"]["controls"]
        .as_array()
        .expect("chat column has controls")
        .iter()
        .map(|control| control["name"].as_str().expect("name is text").to_owned())
        .collect()
}

#[test]
fn chat_chrome_names_appear_in_the_reference_order() {
    let session = reference_names("session", "chatColumn");
    let streaming = reference_names("streaming", "chatColumn");

    assert_eq!(session.last().map(String::as_str), Some(SCROLL_TO_BOTTOM));
    let meta: Vec<&str> = META_ACTIONS.iter().map(|(name, _)| *name).collect();
    assert_eq!(session[session.len() - 5..session.len() - 1], meta[..]);
    assert_eq!(streaming.last().map(String::as_str), Some(SCROLL_TO_BOTTOM));

    // The user message keeps React's hover row: revert, copy, more.
    let user: Vec<&str> = USER_ACTIONS.iter().map(|(name, _)| *name).collect();
    assert_eq!(streaming.first().map(String::as_str), Some(SCROLLER));
    let shown: Vec<&str> = streaming[1..4].iter().map(String::as_str).collect();
    assert_eq!(shown, user);

    // The finished transcript scrolls that message out of React's rendered
    // window, so the idle reference has no user row.
    assert!(!session.iter().any(|name| name == USER_ACTIONS[0].0));
}

#[test]
fn every_fixture_that_shows_a_turn_keeps_the_user_actions() {
    let user: Vec<&str> = USER_ACTIONS.iter().map(|(name, _)| *name).collect();
    // Every captured turn state has one rendered user message; each carries the
    // row, whether the turn is busy, asking or waiting.
    for state in [
        "busy-with-text",
        "form-choice",
        "permission-bash",
        "permission-edit",
        "permission-two",
        "permission-write",
        "queue-one",
        "queue-three-expanded",
        "recovered-question",
    ] {
        let names = turn_names(state);
        assert!(
            names.windows(user.len()).any(|window| window == user),
            "{state} lost the user message's action row: {names:?}"
        );
    }
}
