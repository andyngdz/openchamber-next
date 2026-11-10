//! The model row label React builds.

use super::{Section, header_height, row_height, visible_name};
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
fn favorites_show_the_provider_and_the_context() {
    assert_eq!(
        visible_name(&entry(Some(200_000)), Section::Favorites),
        "Fixture stream, 300 characters/s Performance fixture 200K"
    );
}

#[test]
fn provider_sections_leave_the_provider_out() {
    assert_eq!(
        visible_name(&entry(Some(200_000)), Section::Provider),
        "Fixture stream, 300 characters/s 200K"
    );
}

#[test]
fn the_rows_are_their_padding_plus_one_css_line() {
    // `px-2 py-1.5 typography-meta` (`ModelPickerList.tsx:960`): 6 + 18 + 6.
    assert_eq!(row_height(), 30.);
    // `px-2 py-1.5 typography-micro` (`MobileSheetList.tsx:11`): 6 + 17 + 6.
    assert_eq!(header_height(), 29.);
}
