//! The model row label React builds.

use super::{SectionKind, readable_name};
use oc_state::ModelEntry;

fn entry(context: Option<u64>) -> ModelEntry {
    ModelEntry {
        provider_id: "perf".to_owned(),
        model_id: "stream-300cps".to_owned(),
        name: "Fixture stream, 300 characters/s".to_owned(),
        provider_name: "Performance fixture".to_owned(),
        variants: vec!["low".to_owned(), "high".to_owned()],
        context,
    }
}

#[test]
fn favorites_name_the_provider_and_the_context() {
    assert_eq!(
        readable_name(&entry(Some(200_000)), SectionKind::Favorites),
        "Fixture stream, 300 characters/s Performance fixture 200K"
    );
}

#[test]
fn provider_sections_leave_the_provider_out() {
    assert_eq!(
        readable_name(&entry(Some(200_000)), SectionKind::Provider),
        "Fixture stream, 300 characters/s 200K"
    );
}

#[test]
fn only_favorites_and_recent_headers_carry_a_remix_icon() {
    assert!(SectionKind::Favorites.header_icon().is_some());
    assert!(SectionKind::Recent.header_icon().is_some());
    assert!(SectionKind::Provider.header_icon().is_none());
}
