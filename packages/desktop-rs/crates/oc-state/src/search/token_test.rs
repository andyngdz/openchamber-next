//! Query tokens, item fields and the score one token gets.

use super::{build_rank_fields, score_token, tokenize};

/// A token score close enough to the expected one; the score sums are double arithmetic.
fn assert_score(actual: Option<f64>, expected: f64) {
    let score = actual.expect("a token score");
    assert!((score - expected).abs() < 1e-12, "{score} != {expected}");
}

#[test]
fn a_query_is_trimmed_lowercased_and_split_on_whitespace() {
    let texts: Vec<String> = tokenize("  Chat   Scroll ")
        .into_iter()
        .map(|token| token.text)
        .collect();
    assert_eq!(texts, vec!["chat", "scroll"]);

    let non_ascii: Vec<String> = tokenize("DỰ ÁN")
        .into_iter()
        .map(|token| token.text)
        .collect();
    assert_eq!(non_ascii, vec!["dự", "án"]);

    assert!(tokenize("   ").is_empty());
}

#[test]
fn empty_texts_are_skipped_and_the_rest_are_lowercased() {
    let fields = build_rank_fields(&["", "GLM-5.3", ""]);
    let lowers: Vec<&str> = fields.iter().map(|field| field.lower.as_str()).collect();
    assert_eq!(lowers, vec!["glm-5.3"]);
}

#[test]
fn a_field_prefix_scores_best() {
    let fields = build_rank_fields(&["maintenance"]);
    let tokens = tokenize("main");
    assert_score(score_token(&tokens[0], &fields), 0.0);
}

#[test]
fn a_word_boundary_beats_a_mid_word_match() {
    let tokens = tokenize("main");
    let boundary = build_rank_fields(&["xxx main"]);
    let mid_word = build_rank_fields(&["xxxmain"]);

    assert_score(score_token(&tokens[0], &boundary), 0.1 + 4.0 / 1000.0);
    assert_score(score_token(&tokens[0], &mid_word), 0.2 + 3.0 / 1000.0);
}

#[test]
fn a_later_field_carries_its_field_penalty() {
    let fields = build_rank_fields(&["zzz", "main"]);
    let tokens = tokenize("main");
    assert_score(score_token(&tokens[0], &fields), 0.01);
}

#[test]
fn a_compact_match_is_punctuation_insensitive_and_scores_last() {
    let fields = build_rank_fields(&["gpt-4o-mini"]);
    let tokens = tokenize("gpt4o");
    assert_score(score_token(&tokens[0], &fields), 0.4);
}

#[test]
fn a_token_matches_the_best_of_the_fields() {
    let fields = build_rank_fields(&["zzzmain", "main"]);
    let tokens = tokenize("main");
    assert_score(score_token(&tokens[0], &fields), 0.01);
}

#[test]
fn a_token_missing_every_field_scores_nothing() {
    let fields = build_rank_fields(&["alpha"]);
    let tokens = tokenize("omega");
    assert_eq!(score_token(&tokens[0], &fields), None);
}
