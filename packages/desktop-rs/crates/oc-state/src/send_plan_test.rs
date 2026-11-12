use oc_client::types::Delivery;
use serde_json::json;

use crate::Action;
use crate::FollowUpBehavior;
use crate::SidebarAction;
use crate::fixture_queue_test::{api_queue, api_taken_all};
use crate::fixture_test::{api_permissions, api_settings, directory, event, opened};
use crate::ids::SessionId;
use crate::send::SendKey;
use crate::send_plan::{
    DismissKind, DismissTarget, SendNowPlan, SendPlan, join_queue_text, plan_send_now, taken_texts,
};
use crate::state::State;

/// A busy session with the captured permission pending and two of the captured queue items.
fn busy_session(text: &str, follow_up: FollowUpBehavior) -> (State, SessionId) {
    let requests = api_permissions("permission-request-list");
    let session = requests[0].session_id.clone();
    let mut state = opened(&session);
    state.permissions_loaded(&directory(), requests);
    let mut queue = api_queue("list");
    queue.sessions[0].session_id = session.clone();
    queue.sessions[0].items.truncate(2);
    state.queue_loaded(queue);
    state.apply(Action::Event(event(
        "session.execution.started",
        1,
        json!({"sessionID": session}),
    )));
    if follow_up == FollowUpBehavior::Steer {
        let mut settings = api_settings();
        settings.follow_up_behavior = Some("steer".to_owned());
        state.apply_sidebar(SidebarAction::SettingsLoaded(Box::new(settings)));
    }
    state.apply(Action::DraftChanged(text.to_owned()));
    (state, SessionId::from(session.as_str()))
}

#[test]
fn a_pending_permission_with_an_empty_composer_leaves_the_queue_alone() {
    let (state, session) = busy_session("", FollowUpBehavior::Queue);
    let permission = api_permissions("permission-request-list")[0].id.clone();

    match state.send_plan(SendKey::Plain) {
        SendPlan::DismissThenEnqueue { dismiss, text } => {
            assert_eq!(text, None);
            assert_eq!(
                dismiss,
                vec![DismissTarget {
                    session_id: session.as_str().to_owned(),
                    id: permission,
                    kind: DismissKind::Permission,
                }]
            );
        }
        other => panic!("expected a dismissal that enqueues nothing, got {other:?}"),
    }
    assert_eq!(state.queue_items(&session).len(), 2);
}

#[test]
fn a_dismissing_plain_prompt_enqueues_the_composer_text() {
    let (state, _) = busy_session("hello", FollowUpBehavior::Queue);

    match state.send_plan(SendKey::Command) {
        SendPlan::DismissThenEnqueue { dismiss, text } => {
            assert_eq!(dismiss.len(), 1);
            assert_eq!(text.as_deref(), Some("hello"));
        }
        other => panic!("expected a dismissal that enqueues the text, got {other:?}"),
    }
}

#[test]
fn a_steer_still_sends_after_dismissing_and_downgrades_when_idle() {
    let (state, _) = busy_session("hello", FollowUpBehavior::Steer);

    match state.send_plan(SendKey::Plain) {
        SendPlan::DismissThenSend { dismiss, prompt } => {
            assert_eq!(dismiss.len(), 1);
            assert!(prompt.take_queue);
            assert_eq!(prompt.composer_text, "hello");
            assert!(prompt.steer);
            assert_eq!(prompt.delivery(false), Some(Delivery::Steer));
            assert_eq!(prompt.delivery(true), None);
        }
        other => panic!("expected a steer after the dismissal, got {other:?}"),
    }
}

#[test]
fn queueing_dismisses_nothing() {
    let (state, _) = busy_session("hello", FollowUpBehavior::Queue);

    assert_eq!(
        state.send_plan(SendKey::Plain),
        SendPlan::Enqueue {
            text: "hello".to_owned()
        }
    );
}

#[test]
fn an_empty_composer_with_queued_items_sends_the_whole_queue() {
    let list = api_queue("list");
    let session = SessionId::from(list.sessions[0].session_id.as_str());
    let mut state = opened(session.as_str());
    state.queue_loaded(list);

    let prompt = match state.send_plan(SendKey::Plain) {
        SendPlan::DismissThenSend { dismiss, prompt } => {
            assert!(dismiss.is_empty());
            prompt
        }
        other => panic!("expected the queue to be sent, got {other:?}"),
    };
    assert!(prompt.take_queue);
    assert_eq!(prompt.composer_text, "");
    assert_eq!(prompt.delivery(false), None);

    let blank = "\n".repeat(2);
    let taken = api_taken_all("take-all");
    let expected = taken
        .items
        .iter()
        .map(|item| item.text.as_str())
        .collect::<Vec<_>>()
        .join(&blank);
    assert_eq!(
        join_queue_text(taken_texts(&taken.items), &prompt.composer_text),
        expected
    );
}

#[test]
fn the_send_joins_the_queue_before_the_composer_text() {
    let blank = "\n".repeat(2);
    let taken = api_taken_all("take-all");

    assert_eq!(
        join_queue_text(taken_texts(&taken.items), "and mine"),
        format!(
            "{}{blank}{}{blank}and mine",
            taken.items[0].text, taken.items[1].text
        )
    );
}

#[test]
fn the_join_skips_blank_parts_and_strips_composer_newlines() {
    let blank = "\n".repeat(2);

    assert_eq!(
        join_queue_text(["one", "   "], "two"),
        format!("one{blank}two")
    );
    let composer = format!("{blank}two{blank}");
    assert_eq!(
        join_queue_text(["one"], &composer),
        format!("one{blank}two")
    );
    assert_eq!(
        join_queue_text(["one", "two"], ""),
        format!("one{blank}two")
    );
    assert_eq!(join_queue_text(Vec::<&str>::new(), ""), "");
}

#[test]
fn a_queue_row_sends_now_by_steering_or_a_plain_prompt() {
    let item = api_queue("list").sessions[0].items[0].id.clone();

    assert_eq!(
        plan_send_now(&item, false),
        SendNowPlan {
            item_id: item.clone(),
            delivery: Some(Delivery::Steer),
        }
    );
    assert_eq!(plan_send_now(&item, true).delivery, None);
}

#[test]
fn an_idle_composer_sends_without_taking_a_queue() {
    let mut state = opened("ses_a");
    state.apply(Action::DraftChanged("hello".to_owned()));

    match state.send_plan(SendKey::Plain) {
        SendPlan::DismissThenSend { dismiss, prompt } => {
            assert!(dismiss.is_empty());
            assert!(!prompt.take_queue);
            assert_eq!(prompt.composer_text, "hello");
            assert!(!prompt.steer);
        }
        other => panic!("expected a plain prompt, got {other:?}"),
    }
}

#[test]
fn the_first_send_in_flight_plans_nothing() {
    let (mut state, _) = busy_session("hello", FollowUpBehavior::Queue);
    state.apply(Action::PromptSent);

    assert_eq!(state.send_plan(SendKey::Plain), SendPlan::Nothing);
}

#[test]
fn an_empty_composer_without_a_queue_plans_nothing() {
    assert_eq!(opened("ses_a").send_plan(SendKey::Plain), SendPlan::Nothing);
}
