//! The reconnect loop against fake connects and a fake sleep that records its delays and
//! returns at once, so no test waits on a clock.

use std::cell::RefCell;
use std::time::Duration;

use futures::channel::mpsc;
use futures::stream::{self, BoxStream};
use futures::{Stream, StreamExt};
use oc_client::{ClientError, Event};
use oc_state::{Action, ConnectionStatus};
use serde_json::json;

use super::{FIRST_DELAY, MAX_DELAY, follow_events};
use crate::live::protocol::Update;

type Frames = BoxStream<'static, Result<Event, ClientError>>;

/// What one connect attempt does.
enum Attempt {
    Refused,
    /// Opens, sends these events, then ends.
    Ends(Vec<Event>),
    /// Opens, sends these events, then stays open.
    Stays(Vec<Event>),
}

fn event(id: &str) -> Event {
    serde_json::from_value(json!({"id": id, "type": "server.connected", "data": {}}))
        .expect("event decodes")
}

fn frames(events: Vec<Event>) -> impl Stream<Item = Result<Event, ClientError>> {
    stream::iter(events.into_iter().map(Ok))
}

fn reconnecting(attempt: u32) -> Update {
    Update::Apply(Action::Connection(ConnectionStatus::Reconnecting {
        attempt,
    }))
}

fn connected() -> Update {
    Update::Apply(Action::Connection(ConnectionStatus::Connected))
}

/// Runs the loop over `attempts` until `wanted` updates arrived; returns them and the delays
/// the loop slept. The last attempt must stay open.
async fn run(attempts: Vec<Attempt>, wanted: usize) -> (Vec<Update>, Vec<Duration>) {
    let delays = RefCell::new(Vec::new());
    let script = RefCell::new(attempts.into_iter());
    let connect = || {
        let next = script.borrow_mut().next();
        async move {
            match next {
                Some(Attempt::Refused) => Err(ClientError::Stream("refused".to_owned())),
                Some(Attempt::Ends(events)) => Ok(frames(events).boxed() as Frames),
                Some(Attempt::Stays(events)) => {
                    Ok(frames(events).chain(stream::pending()).boxed() as Frames)
                }
                None => panic!("the loop connected more often than scripted"),
            }
        }
    };
    let sleep = |delay| {
        delays.borrow_mut().push(delay);
        async {}
    };
    let (sender, receiver) = mpsc::channel(8);
    let updates = tokio::select! {
        () = follow_events(connect, sleep, sender) => panic!("the loop stopped on its own"),
        updates = receiver.take(wanted).collect::<Vec<_>>() => updates,
    };
    (updates, delays.into_inner())
}

#[tokio::test]
async fn two_refused_connects_back_off_then_refetch_after_the_reconnect() {
    let attempts = vec![
        Attempt::Refused,
        Attempt::Refused,
        Attempt::Stays(vec![event("evt_1")]),
    ];

    let (updates, delays) = run(attempts, 5).await;

    assert_eq!(
        delays,
        [FIRST_DELAY, Duration::from_secs(1)],
        "500 ms, then doubled"
    );
    assert_eq!(
        updates,
        [
            reconnecting(1),
            reconnecting(2),
            connected(),
            Update::Reconnected,
            Update::Apply(Action::Event(event("evt_1"))),
        ]
    );
}

#[tokio::test]
async fn the_first_connect_does_not_refetch() {
    let (updates, delays) = run(vec![Attempt::Stays(vec![event("evt_1")])], 2).await;

    assert!(delays.is_empty());
    assert_eq!(
        updates,
        [connected(), Update::Apply(Action::Event(event("evt_1")))]
    );
}

#[tokio::test]
async fn a_dropped_stream_reconnects_from_the_first_delay() {
    let attempts = vec![
        Attempt::Refused,
        Attempt::Ends(vec![event("evt_1")]),
        Attempt::Stays(Vec::new()),
    ];

    let (updates, delays) = run(attempts, 7).await;

    assert_eq!(delays, [FIRST_DELAY, FIRST_DELAY]);
    assert_eq!(
        updates,
        [
            reconnecting(1),
            connected(),
            Update::Reconnected,
            Update::Apply(Action::Event(event("evt_1"))),
            reconnecting(1),
            connected(),
            Update::Reconnected,
        ]
    );
}

#[tokio::test]
async fn the_delay_doubles_up_to_ten_seconds() {
    let mut attempts: Vec<Attempt> = (0..7).map(|_| Attempt::Refused).collect();
    attempts.push(Attempt::Stays(Vec::new()));

    let (_, delays) = run(attempts, 9).await;

    let millis: Vec<u128> = delays.iter().map(Duration::as_millis).collect();
    assert_eq!(millis, [500, 1000, 2000, 4000, 8000, 10000, 10000]);
    assert_eq!(delays.last(), Some(&MAX_DELAY));
}
