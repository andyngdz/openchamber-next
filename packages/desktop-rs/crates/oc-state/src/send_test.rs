use crate::EnterKeys;
use crate::FollowUpBehavior;
use crate::SidebarAction;
use crate::fixture_test::{api_settings, directory};
use crate::send::{SendDecision, SendInput, SendKey, decide_send};
use crate::should_submit_enter;
use crate::state::State;

fn input(text: &str, idle: bool, follow_up: FollowUpBehavior, key: SendKey) -> SendInput {
    SendInput {
        text: text.to_owned(),
        idle,
        follow_up,
        key,
        first_send_pending: false,
        has_queue: false,
    }
}

#[test]
fn a_busy_plain_send_queues_or_steers_by_the_setting() {
    assert_eq!(
        decide_send(&input(
            "follow up",
            false,
            FollowUpBehavior::Queue,
            SendKey::Plain
        )),
        SendDecision::Enqueue
    );
    assert_eq!(
        decide_send(&input(
            "follow up",
            false,
            FollowUpBehavior::Steer,
            SendKey::Plain
        )),
        SendDecision::Steer
    );
}

#[test]
fn a_command_send_is_always_a_plain_prompt() {
    for follow_up in [FollowUpBehavior::Queue, FollowUpBehavior::Steer] {
        assert_eq!(
            decide_send(&input("send now", false, follow_up, SendKey::Command)),
            SendDecision::Prompt
        );
    }
}

#[test]
fn an_idle_session_always_sends_a_plain_prompt() {
    for follow_up in [FollowUpBehavior::Queue, FollowUpBehavior::Steer] {
        for key in [SendKey::Plain, SendKey::Command] {
            assert_eq!(
                decide_send(&input("hello", true, follow_up, key)),
                SendDecision::Prompt
            );
        }
    }
}

#[test]
fn the_first_send_in_flight_sends_nothing() {
    let mut pending = input("follow up", false, FollowUpBehavior::Queue, SendKey::Plain);
    pending.first_send_pending = true;
    assert_eq!(decide_send(&pending), SendDecision::None);
}

#[test]
fn an_empty_composer_sends_the_queued_items() {
    for idle in [true, false] {
        let mut queued = input("", idle, FollowUpBehavior::Queue, SendKey::Plain);
        queued.has_queue = true;
        assert_eq!(decide_send(&queued), SendDecision::Prompt);
    }
}

#[test]
fn an_empty_composer_without_a_queue_sends_nothing() {
    assert_eq!(
        decide_send(&input(
            "   ",
            false,
            FollowUpBehavior::Queue,
            SendKey::Plain
        )),
        SendDecision::None
    );
}

#[test]
fn the_settings_document_sets_the_follow_up_behaviour() {
    assert_eq!(
        State::new(directory()).follow_up_behavior(),
        FollowUpBehavior::Queue
    );

    let captured = api_settings();
    assert_eq!(captured.follow_up_behavior(), FollowUpBehavior::Queue);

    let mut state = State::new(directory());
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(captured.clone())));
    assert_eq!(state.follow_up_behavior(), FollowUpBehavior::Queue);

    let mut steer = captured.clone();
    steer.follow_up_behavior = Some("steer".to_owned());
    let mut state = State::new(directory());
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(steer)));
    assert_eq!(state.follow_up_behavior(), FollowUpBehavior::Steer);

    let mut legacy = captured;
    legacy.follow_up_behavior = None;
    legacy.queue_mode_enabled = Some(false);
    let mut state = State::new(directory());
    state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(legacy)));
    assert_eq!(state.follow_up_behavior(), FollowUpBehavior::Steer);
}

#[test]
fn both_enter_settings_reach_the_decision() {
    let settings = api_settings();
    assert!(should_submit_enter(&settings, EnterKeys::default()));
    assert_eq!(
        decide_send(&input(
            "follow up",
            false,
            FollowUpBehavior::Queue,
            SendKey::Plain
        )),
        SendDecision::Enqueue
    );

    let mut ctrl_only = settings;
    ctrl_only.enter_to_send = false;
    assert!(!should_submit_enter(&ctrl_only, EnterKeys::default()));
    assert!(should_submit_enter(
        &ctrl_only,
        EnterKeys {
            ctrl: true,
            ..EnterKeys::default()
        }
    ));
    assert_eq!(
        decide_send(&input(
            "follow up",
            false,
            FollowUpBehavior::Queue,
            SendKey::Command
        )),
        SendDecision::Prompt
    );
}
