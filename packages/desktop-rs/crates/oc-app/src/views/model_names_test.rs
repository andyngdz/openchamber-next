//! Tests for the React display-string ports.

use super::{compact_tokens, effort_label, model_display_name, truncate};

#[test]
fn model_names_are_trimmed_and_cut_at_forty_units() {
    assert_eq!(
        model_display_name("Fixture agent, 20 tool calls then 300 characters/s", "id"),
        "Fixture agent, 20 tool calls then 300..."
    );
    assert_eq!(
        model_display_name("Fixture stream, 1200 characters/s", "id"),
        "Fixture stream, 1200 characters/s"
    );
    assert_eq!(model_display_name("  ", "big-pickle"), "big-pickle");
    assert_eq!(truncate("abcdef", 3), "abc");
}

#[test]
fn compact_tokens_matches_intl_compact_notation() {
    assert_eq!(compact_tokens(999), "999");
    assert_eq!(compact_tokens(1_000), "1K");
    assert_eq!(compact_tokens(1_500), "1.5K");
    assert_eq!(compact_tokens(12_345), "12.3K");
    assert_eq!(compact_tokens(200_000), "200K");
    assert_eq!(compact_tokens(262_144), "262.1K");
    assert_eq!(compact_tokens(1_000_000), "1M");
    assert_eq!(compact_tokens(1_048_576), "1M");
    assert_eq!(compact_tokens(2_500_000), "2.5M");
}

#[test]
fn effort_labels_capitalize_the_level() {
    assert_eq!(effort_label(None), "Default");
    assert_eq!(effort_label(Some("high")), "High");
    assert_eq!(effort_label(Some("low")), "Low");
    assert_eq!(effort_label(Some("3")), "3");
}
