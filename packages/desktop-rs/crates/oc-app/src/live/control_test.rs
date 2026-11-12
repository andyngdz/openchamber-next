//! The control stream against fake connects, a fake sleep that records its delays, and a
//! scripted HTTP server for the ready-frame reads.

use std::cell::RefCell;
use std::time::Duration;

use futures::StreamExt;
use futures::channel::mpsc;
use futures::stream::{self, BoxStream};
use oc_client::ClientError;
use oc_client::control::ControlEvent;

use super::{FIRST_DELAY, MAX_DELAY, follow, refresh};
use crate::live::fake_server_test::FakeServer;
use crate::live::protocol::Update;

type Frames = BoxStream<'static, Result<ControlEvent, ClientError>>;

/// What one connect attempt does.
enum Attempt {
    Refused,
    Missing,
    /// Opens, sends these frames, then ends.
    Ends(Vec<ControlEvent>),
    /// Opens, sends these frames, then stays open.
    Stays(Vec<ControlEvent>),
}

fn frames(
    events: Vec<ControlEvent>,
) -> impl futures::Stream<Item = Result<ControlEvent, ClientError>> {
    stream::iter(events.into_iter().map(Ok))
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
                Some(Attempt::Missing) => Err(ClientError::NotFound {
                    body: String::new(),
                }),
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
        () = follow(connect, sleep, sender) => panic!("the loop stopped on its own"),
        updates = receiver.take(wanted).collect::<Vec<_>>() => updates,
    };
    (updates, delays.into_inner())
}

#[tokio::test]
async fn a_missing_route_stops_the_stream_without_a_retry() {
    let delays = RefCell::new(Vec::new());
    let script = RefCell::new(vec![Attempt::Missing].into_iter());
    let connect = || {
        let next = script.borrow_mut().next();
        async move {
            match next {
                Some(Attempt::Missing) => Err::<Frames, ClientError>(ClientError::NotFound {
                    body: String::new(),
                }),
                _ => panic!("a 404 must not reconnect"),
            }
        }
    };
    let sleep = |delay| {
        delays.borrow_mut().push(delay);
        async {}
    };
    let (sender, receiver) = mpsc::channel(8);

    follow(connect, sleep, sender).await;
    let updates: Vec<Update> = receiver.collect().await;

    assert!(delays.into_inner().is_empty(), "a 404 is not retried");
    assert_eq!(updates, [Update::ControlStreamMissing]);
}

#[tokio::test]
async fn a_refused_connect_backs_off_from_one_second() {
    let attempts = vec![
        Attempt::Refused,
        Attempt::Refused,
        Attempt::Stays(vec![ControlEvent::Ready]),
    ];

    let (updates, delays) = run(attempts, 1).await;

    assert_eq!(delays, [FIRST_DELAY, Duration::from_secs(2)]);
    assert_eq!(updates, [Update::Control(ControlEvent::Ready)]);
}

#[tokio::test]
async fn a_dropped_stream_reconnects_from_the_first_delay() {
    let attempts = vec![
        Attempt::Ends(vec![ControlEvent::Heartbeat]),
        Attempt::Stays(vec![ControlEvent::Ready]),
    ];

    let (updates, delays) = run(attempts, 2).await;

    assert_eq!(delays, [FIRST_DELAY]);
    assert_eq!(
        updates,
        [
            Update::Control(ControlEvent::Heartbeat),
            Update::Control(ControlEvent::Ready),
        ]
    );
}

#[tokio::test]
async fn the_delay_doubles_up_to_thirty_seconds() {
    let mut attempts: Vec<Attempt> = (0..6).map(|_| Attempt::Refused).collect();
    attempts.push(Attempt::Stays(vec![ControlEvent::Ready]));

    let (_, delays) = run(attempts, 1).await;

    let millis: Vec<u128> = delays.iter().map(Duration::as_millis).collect();
    assert_eq!(millis, [1000, 2000, 4000, 8000, 16000, 30000]);
    assert_eq!(delays.last(), Some(&MAX_DELAY));
}

const QUEUE: &str = r#"{"revision":3,"sessions":[]}"#;
const MODES: &str = r#"{"revision":2,"modes":{}}"#;
const ROUTING: &str = r#"{"available":true,"jevAvailable":false,"heldPermissions":[]}"#;

#[tokio::test]
async fn a_ready_frame_reads_the_queue_modes_and_routing() {
    let server = FakeServer::start(vec![
        ("200 OK", QUEUE),
        ("200 OK", MODES),
        ("200 OK", ROUTING),
    ]);
    let (mut sender, receiver) = mpsc::channel(8);

    refresh(&server.client, &mut sender)
        .await
        .expect("window open");
    drop(sender);
    let updates: Vec<Update> = receiver.collect().await;
    let paths: Vec<String> = server
        .finish()
        .into_iter()
        .map(|request| request.path)
        .collect();

    assert_eq!(
        paths,
        [
            "/api/message-queue",
            "/api/permission-auto-accept",
            "/api/routing"
        ]
    );
    let [
        Update::QueueLoaded(Ok(queue)),
        Update::AutoAcceptLoaded(Ok(modes)),
        Update::RoutingLoaded(Ok(routing)),
    ] = updates.as_slice()
    else {
        panic!("three snapshots expected, got {updates:?}");
    };
    assert_eq!(queue.revision, 3);
    assert_eq!(modes.revision, 2);
    assert!(routing.available);
    assert!(!routing.jev_available);
}

#[tokio::test]
async fn a_failed_read_keeps_its_own_error_and_the_others_apply() {
    let server = FakeServer::start(vec![
        ("500 Internal Server Error", "{}"),
        ("200 OK", MODES),
        ("200 OK", ROUTING),
    ]);
    let (mut sender, receiver) = mpsc::channel(8);

    refresh(&server.client, &mut sender)
        .await
        .expect("window open");
    drop(sender);
    let updates: Vec<Update> = receiver.collect().await;
    drop(server.finish());

    let [
        Update::QueueLoaded(Err(message)),
        Update::AutoAcceptLoaded(Ok(_)),
        Update::RoutingLoaded(Ok(_)),
    ] = updates.as_slice()
    else {
        panic!("one failed read and two snapshots expected, got {updates:?}");
    };
    assert!(message.contains("500"), "{message}");
}
