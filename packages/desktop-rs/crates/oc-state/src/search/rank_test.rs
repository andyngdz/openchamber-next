//! `rank_by_query` and `matches_rank_query`: the cases of `fuzzySearch.test.ts` that apply, plus
//! the generated React output in `fixtures/search/rank.json`.
//!
//! The sidebar's `wordStart` option (`SIDEBAR_SEARCH`, `fuzzySearch.ts:186-205`) is not ported:
//! this change has no sidebar search (design.md "Search" names only the two functions above).

use serde::Deserialize;

use super::super::fixture_test::read;
use super::{RankByQueryOptions, matches_rank_query, rank_by_query};

fn rank<'a>(items: &'a [&'a str], query: &str) -> Vec<&'a str> {
    rank_by_query(
        items,
        query,
        |item| vec![*item],
        RankByQueryOptions::default(),
    )
    .into_iter()
    .copied()
    .collect()
}

#[test]
fn word_boundary_matches_rank_above_mid_word_matches() {
    let items = ["prefixed-thing", "workspace-fix", "feat/fix-scroll"];
    assert_eq!(
        rank(&items, "fix"),
        vec!["feat/fix-scroll", "workspace-fix", "prefixed-thing"]
    );
}

#[test]
fn an_exact_prefix_comes_first_and_ties_keep_input_order() {
    let items = ["main", "feat/main-menu", "maintenance", "release/main"];
    assert_eq!(
        rank(&items, "main"),
        vec!["main", "maintenance", "feat/main-menu", "release/main"]
    );
}

#[test]
fn multi_token_queries_match_in_any_order_and_need_every_token() {
    let items = [
        "feat/scroll-anchored-chat",
        "fix/chat-header",
        "feat/scroll-perf",
    ];
    assert_eq!(
        rank(&items, "chat scroll"),
        vec!["feat/scroll-anchored-chat"]
    );
}

#[test]
fn punctuation_insensitive_compact_matching_finds_joined_words() {
    let items = ["gpt-4o-mini", "claude-sonnet-5"];
    assert_eq!(rank(&items, "gpt4o"), vec!["gpt-4o-mini"]);
    assert_eq!(rank(&items, "sonnet5"), vec!["claude-sonnet-5"]);
}

#[test]
fn punctuation_only_tokens_match_nothing_instead_of_everything() {
    let options = RankByQueryOptions {
        fuzzy: false,
        ..RankByQueryOptions::default()
    };
    assert!(rank_by_query(&["alpha", "beta"], "...", |item| vec![*item], options).is_empty());
    assert!(rank(&["alpha", "beta"], "--").is_empty());
}

#[test]
fn single_token_queries_tolerate_typos_via_the_fuzzy_fallback() {
    let items = ["workspace-rail-layout", "unrelated"];
    assert_eq!(rank(&items, "worskpace"), vec!["workspace-rail-layout"]);
}

#[test]
fn the_fuzzy_fallback_can_be_disabled() {
    let options = RankByQueryOptions {
        fuzzy: false,
        ..RankByQueryOptions::default()
    };
    let ranked = rank_by_query(
        &["workspace-rail-layout"],
        "worskpace",
        |item| vec![*item],
        options,
    );
    assert!(ranked.is_empty());
}

#[test]
fn earlier_fields_outrank_later_fields() {
    #[derive(Clone, Copy)]
    struct Item<'a> {
        name: &'a str,
        path: &'a str,
    }
    let items = [
        Item {
            name: "docs",
            path: "/repo/build-agent",
        },
        Item {
            name: "build-agent",
            path: "/repo/build-agent",
        },
    ];
    let ranked = rank_by_query(
        &items,
        "build",
        |item| vec![item.name, item.path],
        RankByQueryOptions::default(),
    );
    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].name, "build-agent");
}

#[test]
fn an_empty_query_returns_the_items_unchanged_within_the_limit() {
    assert_eq!(rank(&["b", "a"], "  "), vec!["b", "a"]);

    let options = RankByQueryOptions {
        limit: Some(2),
        ..RankByQueryOptions::default()
    };
    assert_eq!(
        rank_by_query(&["a", "b", "c"], "", |item| vec![*item], options),
        vec![&"a", &"b"]
    );
}

#[test]
fn non_ascii_text_ranks_as_javascript_ranks_it() {
    // Compact matching drops the diacritics, so "dự" and "án" compact to "d" and "n".
    assert_eq!(
        rank(&["Dự án OpenChamber", "Ứng dụng trò chuyện"], "DỰ ÁN"),
        vec!["Dự án OpenChamber", "Ứng dụng trò chuyện"]
    );
    assert_eq!(
        rank(&["Dự án OpenChamber", "alpha"], "dự án"),
        vec!["Dự án OpenChamber"]
    );
    // Positions count UTF-16 code units: three astral characters move the match to index 7,
    // behind the ASCII item, while a code-point port would rank it first.
    assert_eq!(
        rank(&["🚀🚀🚀 x", "abcd x"], "x"),
        vec!["abcd x", "🚀🚀🚀 x"]
    );
}

#[test]
fn every_token_must_match_across_the_fields() {
    assert!(matches_rank_query(&["GLM-5.3", "Zhipu"], "zhipu glm"));
    assert!(!matches_rank_query(&["GLM-5.3", "Zhipu"], "zhipu gpt"));
}

#[test]
fn matching_is_punctuation_insensitive_and_skips_empty_fields() {
    assert!(matches_rank_query(&["", "claude-sonnet-5", ""], "sonnet5"));
    assert!(!matches_rank_query(&[""], "a"));
}

#[test]
fn matching_rejects_a_punctuation_only_query() {
    assert!(!matches_rank_query(&["alpha"], "..."));
    assert!(!matches_rank_query(&["alpha"], "... alpha"));
}

#[test]
fn fixture_results_match_react() {
    #[derive(Deserialize)]
    struct Fixture {
        corpus: Vec<String>,
        queries: Vec<Query>,
    }

    #[derive(Deserialize)]
    struct Query {
        query: String,
        ranked: Vec<usize>,
        matched: Vec<bool>,
    }

    let fixture: Fixture = read("search/rank.json");
    let items = fixture.corpus.clone();
    for query in &fixture.queries {
        let actual: Vec<usize> = rank_by_query(
            &items,
            &query.query,
            |item| vec![item.as_str()],
            RankByQueryOptions::default(),
        )
        .into_iter()
        .map(|item| {
            items
                .iter()
                .position(|entry| entry == item)
                .expect("a ranked item from the corpus")
        })
        .collect();
        assert_eq!(actual, query.ranked, "ranking for {:?}", query.query);

        let matched: Vec<bool> = fixture
            .corpus
            .iter()
            .map(|item| matches_rank_query(&[item.as_str()], &query.query))
            .collect();
        assert_eq!(matched, query.matched, "matching for {:?}", query.query);
    }
}
