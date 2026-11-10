//! The Bitap searcher: exact matches, typos, the match-length floor and pattern chunking.

use super::BitapSearch;

#[test]
fn an_exact_text_scores_zero() {
    assert_eq!(BitapSearch::new("MAIN").search_in("main"), Some(0.0));
}

#[test]
fn a_typo_scores_the_share_of_wrong_units() {
    // Fuse.js 7.1.0 reports 2/9 for this pair.
    assert_eq!(
        BitapSearch::new("worskpace").search_in("workspace-fix"),
        Some(2.0 / 9.0)
    );
}

#[test]
fn an_unrelated_text_does_not_match() {
    assert_eq!(BitapSearch::new("zzzz").search_in("alpha"), None);
}

#[test]
fn a_single_unit_pattern_needs_a_run_of_two() {
    assert_eq!(BitapSearch::new("a").search_in("alpha"), None);
}

#[test]
fn a_pattern_longer_than_one_chunk_is_averaged_over_its_chunks() {
    let text = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnop";
    let pattern = &text[..40];
    // Two exact chunks, each floored to 0.001.
    assert_eq!(BitapSearch::new(pattern).search_in(text), Some(0.001));
    // The second chunk is the last 32 units and holds the typo once.
    let typo = format!("{}x{}", &text[..20], &text[21..40]);
    assert_eq!(
        BitapSearch::new(typo.as_str()).search_in(text),
        Some(0.031_25)
    );
}
