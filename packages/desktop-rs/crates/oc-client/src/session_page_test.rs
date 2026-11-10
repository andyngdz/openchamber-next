use serde_json::json;

use super::{PageStep, SessionPager};
use crate::types::{Page, Session};

const FIXTURES: &str = concat!(env!("OC_DESKTOP_FIXTURES"), "/api");
const FIRST_NEXT: &str = "eyJhbmNob3IiOnsiaWQiOiJzZXNfZTNiMDRhMjVlZmZlNFdpb3RQTUcwMTVxT1oiLCJ0aW1lIjoxNzk0MzA2MTcwMjgyLCJkaXJlY3Rpb24iOiJuZXh0In19";
const SECOND_NEXT: &str = "eyJhbmNob3IiOnsiaWQiOiJzZXNfZTNiMDRhMmE4ZmZleGp5VlJTMTAzdDhwYXUiLCJ0aW1lIjoxNzk0MzA2MTcwMjA2LCJkaXJlY3Rpb24iOiJuZXh0In19";

fn page(ids: &[&str], next: Option<&str>) -> Page<Session> {
    let data: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| json!({"id": id, "location": {"directory": "/tmp/p"}}))
        .collect();
    serde_json::from_value(json!({"data": data, "cursor": {"next": next}})).expect("page")
}

fn fixture(name: &str) -> Page<Session> {
    let path = format!("{FIXTURES}/{name}");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("decode {name}: {err}"))
}

#[test]
fn a_page_without_a_next_cursor_ends_the_walk() {
    let mut pager = SessionPager::new();

    assert_eq!(pager.accept(page(&["ses_a"], None)), PageStep::Done);
    assert_eq!(pager.cursor(), None);
    assert_eq!(pager.sessions().len(), 1);
}

#[test]
fn a_next_cursor_that_repeats_the_requested_one_ends_the_walk() {
    let mut pager = SessionPager::new();
    assert_eq!(
        pager.accept(page(&["ses_a"], Some("cursor-1"))),
        PageStep::Next("cursor-1".to_owned())
    );

    assert_eq!(
        pager.accept(page(&["ses_b"], Some("cursor-1"))),
        PageStep::Done
    );
    assert_eq!(pager.cursor(), None);
}

#[test]
fn a_page_that_adds_no_new_id_ends_the_walk() {
    let mut pager = SessionPager::new();
    pager.accept(page(&["ses_a"], Some("cursor-1")));

    assert_eq!(
        pager.accept(page(&["ses_a"], Some("cursor-2"))),
        PageStep::Done
    );
}

#[test]
fn sessions_are_deduped_across_pages_in_server_order() {
    let mut pager = SessionPager::new();
    pager.accept(page(&["ses_a", "ses_b"], Some("cursor-1")));

    assert_eq!(
        pager.accept(page(&["ses_b", "ses_c", "ses_a"], Some("cursor-2"))),
        PageStep::Next("cursor-2".to_owned())
    );
    let ids: Vec<String> = pager
        .into_sessions()
        .into_iter()
        .map(|session| session.id)
        .collect();
    assert_eq!(ids, ["ses_a", "ses_b", "ses_c"]);
}

#[test]
fn the_captured_global_pages_walk_to_their_last_cursor() {
    let mut pager = SessionPager::new();

    assert_eq!(
        pager.accept(fixture("session-list-global-page-1.json")),
        PageStep::Next(FIRST_NEXT.to_owned())
    );
    assert_eq!(
        pager.accept(fixture("session-list-global-page-2.json")),
        PageStep::Next(SECOND_NEXT.to_owned())
    );
    assert_eq!(
        pager.accept(fixture("session-list-global-page-2.json")),
        PageStep::Done,
        "a repeated page adds no id"
    );
    assert_eq!(pager.sessions().len(), 4);
}
