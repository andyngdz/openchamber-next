//! Tests for the version 2 state parser.

use super::parse;
use crate::fixture::fixture_test::state_path;

#[test]
fn a_version_two_fixture_builds_the_seed_state() {
    let text = std::fs::read_to_string(state_path("empty")).expect("fixture reads");
    let (state, open) = parse(&text).expect("fixture parses");

    assert!(open.is_none());
    assert_eq!(
        state
            .settings()
            .and_then(|s| s.projects.first())
            .and_then(|p| p.label()),
        Some("desktop-rs-reference")
    );
    assert!(state.global_sessions().len() >= 6);
    assert!(state.draft().selection.model().is_some());
}

#[test]
fn another_version_is_a_decode_error() {
    let text = serde_json::json!({
        "version": 1,
        "home": { "home": "/tmp/home" },
        "settings": {}
    })
    .to_string();

    let error = parse(&text).expect_err("version 1 is rejected");

    assert!(error.to_string().contains("version 1"));
}

#[test]
fn an_unknown_popup_is_a_decode_error() {
    let text = serde_json::json!({
        "version": 2,
        "home": { "home": "/tmp/home" },
        "settings": {},
        "open": "menu"
    })
    .to_string();

    assert!(parse(&text).is_err());
}
