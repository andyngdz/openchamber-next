use oc_client::control::ControlEvent;
use oc_client::queue::QueueUpdate;

use crate::fixture_queue_test::{api_queue, api_queue_update};
use crate::fixture_test::{control_events, opened};
use crate::ids::SessionId;

fn captured_session(queue: &oc_client::queue::MessageQueue) -> SessionId {
    SessionId::from(queue.sessions[0].session_id.as_str())
}

#[test]
fn the_captured_list_fills_the_session_at_its_revision() {
    let list = api_queue("list");
    let expected: Vec<String> = list.sessions[0]
        .items
        .iter()
        .map(|item| item.text.clone())
        .collect();
    let session = captured_session(&list);
    let mut state = opened(session.as_str());

    state.queue_loaded(list);

    let texts: Vec<&str> = state
        .queue_items(&session)
        .iter()
        .map(|item| item.text.as_str())
        .collect();
    assert_eq!(
        texts,
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(state.queue_revision(&session), Some(3));
}

#[test]
fn a_queue_snapshot_is_skipped_only_when_its_revision_is_older() {
    let list = api_queue("list");
    let session = captured_session(&list);
    let mut state = opened(session.as_str());
    state.queue_loaded(list.clone());

    let reorder = api_queue_update("reorder");
    state.queue_updated(&reorder);
    assert_eq!(state.queue_revision(&session), Some(4));
    assert_eq!(
        state.queue_items(&session)[0].id,
        reorder.session.items[0].id
    );

    let older = control_events()
        .into_iter()
        .find_map(|event| match event {
            ControlEvent::MessageQueueUpdated(update) if update.revision == 2 => Some(update),
            _ => None,
        })
        .expect("the capture broadcasts revision 2");
    state.queue_updated(&older);
    assert_eq!(state.queue_revision(&session), Some(4));
    assert_eq!(
        state.queue_items(&session)[0].id,
        reorder.session.items[0].id
    );

    // Equal revisions apply: an action's answer and its broadcast share one.
    let equal = QueueUpdate {
        revision: reorder.revision,
        session: list.sessions[0].clone(),
    };
    state.queue_updated(&equal);
    assert_eq!(state.queue_revision(&session), Some(4));
    assert_eq!(
        state.queue_items(&session)[0].id,
        list.sessions[0].items[0].id
    );
}

#[test]
fn a_full_get_replaces_every_session_at_its_revision() {
    let list = api_queue("list");
    let session = captured_session(&list);
    let mut state = opened(session.as_str());

    let mut extra = list.sessions[0].clone();
    extra.session_id = "ses_extra".to_owned();
    let extra_id = SessionId::from("ses_extra");
    state.queue_updated(&QueueUpdate {
        revision: 2,
        session: extra,
    });
    assert!(state.queue(&extra_id).is_some());

    state.queue_loaded(list.clone());
    assert!(state.queue(&extra_id).is_none());

    // A broadcast past the GET's revision survives a later, older GET.
    state.queue_updated(&api_queue_update("remove"));
    assert!(state.queue_items(&session).is_empty());
    state.queue_loaded(api_queue("list"));
    assert!(state.queue_items(&session).is_empty());
    assert_eq!(state.queue_revision(&session), Some(8));
}

#[test]
fn the_item_the_server_is_sending_is_marked() {
    let in_flight = control_events()
        .into_iter()
        .find_map(|event| match event {
            ControlEvent::MessageQueueUpdated(update) if update.session.sending_id.is_some() => {
                Some(update)
            }
            _ => None,
        })
        .expect("the capture broadcasts one in-flight item");
    let session = SessionId::from(in_flight.session.session_id.as_str());
    let item_id = in_flight.session.items[0].id.clone();
    let mut state = opened(session.as_str());

    state.queue_updated(&in_flight);

    assert!(state.queue_is_sending(&session, &item_id));
    assert!(!state.queue_is_sending(&session, "queued-other"));
}
