//! `followUpBehavior` and the retired `queueModeEnabled` mapping.

use super::FollowUpBehavior;
use crate::settings::Settings;

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");

fn settings(json: &str) -> Settings {
    serde_json::from_str(json).expect("settings")
}

#[test]
fn the_captured_settings_leave_the_send_queued() {
    let path = format!("{FIXTURES}/settings.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    let settings: Settings = serde_json::from_str(&text).expect("settings");
    assert_eq!(settings.follow_up_behavior(), FollowUpBehavior::Queue);
}

#[test]
fn the_current_key_wins() {
    assert_eq!(
        settings(r#"{"followUpBehavior":"steer"}"#).follow_up_behavior(),
        FollowUpBehavior::Steer
    );
    assert_eq!(
        settings(r#"{"followUpBehavior":"queue"}"#).follow_up_behavior(),
        FollowUpBehavior::Queue
    );
    assert_eq!(
        settings(r#"{"followUpBehavior":"steer","queueModeEnabled":true}"#).follow_up_behavior(),
        FollowUpBehavior::Steer,
        "the retired key is read only when the current one is absent"
    );
}

#[test]
fn the_removed_immediate_value_collapses_onto_steer() {
    assert_eq!(
        settings(r#"{"followUpBehavior":"immediate"}"#).follow_up_behavior(),
        FollowUpBehavior::Steer
    );
}

#[test]
fn the_retired_on_off_key_maps_onto_the_behaviour() {
    assert_eq!(
        settings(r#"{"queueModeEnabled":false}"#).follow_up_behavior(),
        FollowUpBehavior::Steer
    );
    assert_eq!(
        settings(r#"{"queueModeEnabled":true}"#).follow_up_behavior(),
        FollowUpBehavior::Queue
    );
}

#[test]
fn an_unknown_key_falls_through_to_the_retired_one_and_then_the_default() {
    assert_eq!(
        settings(r#"{"followUpBehavior":"later"}"#).follow_up_behavior(),
        FollowUpBehavior::Queue
    );
    assert_eq!(
        settings(r#"{"followUpBehavior":"later","queueModeEnabled":false}"#).follow_up_behavior(),
        FollowUpBehavior::Steer
    );
}
