//! The match mask: a run has to reach `minMatchCharLength` for a chunk to count as matched.

use super::MatchMask;

#[test]
fn an_empty_mask_has_no_run() {
    assert!(!MatchMask::new(4).has_run(2));
}

#[test]
fn a_run_of_two_counts_but_a_run_of_three_does_not() {
    let mut mask = MatchMask::new(4);
    mask.set(1, 1);
    mask.set(2, 1);
    assert!(mask.has_run(2));
    assert!(!mask.has_run(3));
}

#[test]
fn separate_matches_do_not_join_into_a_run() {
    let mut mask = MatchMask::new(4);
    mask.set(0, 1);
    mask.set(2, 1);
    assert!(!mask.has_run(2));
}

#[test]
fn a_run_that_reaches_the_end_counts() {
    let mut mask = MatchMask::new(4);
    mask.set(2, 1);
    mask.set(3, 1);
    assert!(mask.has_run(2));
}

#[test]
fn a_run_past_the_text_is_kept() {
    let mut mask = MatchMask::new(2);
    mask.set(4, 1);
    mask.set(5, 1);
    assert!(mask.has_run(2));
}
