use super::{META_ACTIONS, SCROLL_TO_BOTTOM};
use crate::views::reference_test::names as reference_names;

#[test]
fn chat_chrome_names_appear_in_the_reference_order() {
    let session = reference_names("session", "chatColumn");
    let streaming = reference_names("streaming", "chatColumn");

    assert_eq!(session.last().map(String::as_str), Some(SCROLL_TO_BOTTOM));
    let meta: Vec<&str> = META_ACTIONS.iter().map(|(name, _)| *name).collect();
    assert_eq!(session[session.len() - 5..session.len() - 1], meta[..]);
    assert_eq!(streaming.last().map(String::as_str), Some(SCROLL_TO_BOTTOM));
}
