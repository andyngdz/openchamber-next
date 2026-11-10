use crate::live::protocol::Update;
use futures::StreamExt;
use futures::channel::mpsc;
use oc_state::SidebarAction;

use super::load_global;
use crate::live::fake_server_test::FakeServer;

const PAGE: &str =
    r#"{"data":[{"id":"ses_0","location":{"directory":"/p"}}],"cursor":{"next":"c1"}}"#;
const LAST: &str = r#"{"data":[{"id":"ses_1","location":{"directory":"/p"}}],"cursor":{}}"#;
const EMPTY: &str = r#"{"data":[],"cursor":{}}"#;

async fn run(replies: Vec<(&'static str, &'static str)>) -> (Vec<Update>, usize) {
    let server = FakeServer::start(replies);
    let (mut sender, receiver) = mpsc::channel(8);
    load_global(&server.client, &mut sender)
        .await
        .expect("window open");
    drop(sender);
    let updates: Vec<Update> = receiver.collect().await;
    let count = server.finish().len();
    (updates, count)
}

fn page_loaded(sessions: usize) -> Update {
    Update::Sidebar(SidebarAction::PageLoaded(
        (0..sessions)
            .map(|index| {
                serde_json::from_value(serde_json::json!({
                    "id": format!("ses_{index}"),
                    "location": {"directory": "/p"}
                }))
                .expect("session")
            })
            .collect(),
    ))
}

fn complete(sessions: usize) -> Update {
    Update::Sidebar(SidebarAction::Complete(
        (0..sessions)
            .map(|index| {
                serde_json::from_value(serde_json::json!({
                    "id": format!("ses_{index}"),
                    "location": {"directory": "/p"}
                }))
                .expect("session")
            })
            .collect(),
    ))
}

#[tokio::test]
async fn the_first_page_is_shown_and_then_replaced_by_the_complete_list() {
    let (updates, requests) = run(vec![("200 OK", PAGE), ("200 OK", LAST)]).await;

    assert_eq!(updates, [page_loaded(1), complete(2)]);
    assert_eq!(requests, 2);
}

#[tokio::test]
async fn a_first_page_fails_after_three_tries() {
    let failure = ("500 Internal Server Error", "{}");
    let (updates, requests) = run(vec![failure, failure, failure]).await;

    let [Update::Sidebar(SidebarAction::PageFailed(message))] = updates.as_slice() else {
        panic!("a page failure expected, got {updates:?}");
    };
    assert!(message.contains("500"), "{message}");
    assert_eq!(requests, 3);
}

#[tokio::test]
async fn a_page_that_succeeds_on_a_retry_keeps_the_walk() {
    let failure = ("500 Internal Server Error", "{}");
    let (updates, requests) = run(vec![failure, ("200 OK", EMPTY)]).await;

    assert_eq!(updates, [page_loaded(0), complete(0)]);
    assert_eq!(requests, 2);
}

#[tokio::test]
async fn a_later_page_failure_keeps_the_first_page_shown() {
    let failure = ("500 Internal Server Error", "{}");
    let (updates, requests) = run(vec![("200 OK", PAGE), failure, failure, failure]).await;

    let [
        Update::Sidebar(SidebarAction::PageLoaded(_)),
        Update::Sidebar(SidebarAction::PageFailed(message)),
    ] = updates.as_slice()
    else {
        panic!("an incomplete walk expected, got {updates:?}");
    };
    assert!(message.contains("500"), "{message}");
    assert_eq!(requests, 4);
}
