//! The list ranking the pickers use, ported from `rankByQuery` and `matchesRankQuery`
//! (`packages/ui/src/lib/search/fuzzySearch.ts`).
//!
//! A query is trimmed, lowercased and split on whitespace. Every token must match in one of an
//! item's fields, and an item's score is the sum of its best token scores: earlier fields and
//! earlier positions score better, a field prefix beats a word-boundary substring, which beats a
//! mid-word substring, which beats a punctuation-insensitive "compact" match. Ties keep the input
//! order. A single token of three or more code units that no item matched falls back to Fuse's
//! typo-tolerant search, with its score shifted above every token score.
//!
//! Everything is measured in UTF-16 code units, as JavaScript measures it, so a non-ASCII label
//! ranks as it does in React; see [`super::text`] and [`super::token`].

use std::cmp::Ordering;

use super::fuse;
use super::text::unit_len;
use super::token::{RankField, Token, build_rank_fields, score_token, tokenize};

/// Options for [`rank_by_query`], mirroring the object `rankByQuery` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankByQueryOptions {
    /// Maximum number of items returned; `None` (the default) returns every match.
    pub limit: Option<usize>,
    /// Whether a single token of three or more code units falls back to Fuse; `true` by default.
    pub fuzzy: bool,
}

impl Default for RankByQueryOptions {
    fn default() -> Self {
        Self {
            limit: None,
            fuzzy: true,
        }
    }
}

/// Orders `items` by how well they match `query`, the way the React pickers order theirs.
///
/// `get_texts` returns an item's searchable texts, most important first; an empty text is
/// skipped, as a null one is. An empty query returns the items in their original order.
///
/// # Examples
///
/// ```
/// use oc_state::search::{RankByQueryOptions, rank_by_query};
///
/// let items = ["prefixed-thing", "workspace-fix", "feat/fix-scroll"];
/// let ranked: Vec<&str> = rank_by_query(&items, "fix", |item| vec![*item], RankByQueryOptions::default())
///     .into_iter()
///     .copied()
///     .collect();
///
/// assert_eq!(ranked, vec!["feat/fix-scroll", "workspace-fix", "prefixed-thing"]);
/// ```
pub fn rank_by_query<'a, T>(
    items: &'a [T],
    query: &str,
    get_texts: impl Fn(&T) -> Vec<&str>,
    options: RankByQueryOptions,
) -> Vec<&'a T> {
    let tokens = tokenize(query);
    let limit = options.limit.unwrap_or(items.len());
    if tokens.is_empty() {
        return items.iter().take(limit).collect();
    }
    let mut scored = Vec::new();
    let mut missed = Vec::new();
    for (order, item) in items.iter().enumerate() {
        let fields = build_rank_fields(&get_texts(item));
        let total = tokens.iter().try_fold(0.0, |total, token| {
            score_token(token, &fields).map(|score| total + score)
        });
        match total {
            Some(score) => scored.push(Scored { item, score, order }),
            None => missed.push(Missed {
                item,
                joined: joined_fields(&fields),
                order,
            }),
        }
    }
    if options.fuzzy && tokens.len() == 1 && unit_len(&tokens[0].text) >= 3 && !missed.is_empty() {
        add_fuzzy_matches(&tokens[0], &missed, &mut scored);
    }
    scored.sort_by(compare_scores);
    scored
        .into_iter()
        .take(limit)
        .map(|entry| entry.item)
        .collect()
}

/// Whether every token matches one of `texts`, without the fuzzy fallback. Lists that keep their
/// own grouping or order use this, as the model picker does.
///
/// The sidebar's `wordStart` option (`SIDEBAR_SEARCH`) is not ported: the desktop sidebar has no
/// search.
///
/// An empty text is skipped, as a null one is.
///
/// # Examples
///
/// ```
/// use oc_state::search::matches_rank_query;
///
/// assert!(matches_rank_query(&["GLM-5.3", "Zhipu"], "zhipu glm"));
/// assert!(!matches_rank_query(&["GLM-5.3", "Zhipu"], "zhipu gpt"));
/// ```
pub fn matches_rank_query(texts: &[&str], query: &str) -> bool {
    let tokens = tokenize(query);
    if tokens.is_empty() {
        return true;
    }
    let fields = build_rank_fields(texts);
    tokens
        .iter()
        .all(|token| score_token(token, &fields).is_some())
}

/// An item with its token score and its input position.
struct Scored<'a, T> {
    item: &'a T,
    score: f64,
    order: usize,
}

/// An item no token matched, kept for the fuzzy fallback with its joined fields.
struct Missed<'a, T> {
    item: &'a T,
    joined: String,
    order: usize,
}

/// The fields as `Fuse` indexes them: lowercased and joined with one space.
fn joined_fields(fields: &[RankField]) -> String {
    let mut joined = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            joined.push(' ');
        }
        joined.push_str(&field.lower);
    }
    joined
}

/// Adds Fuse's typo-tolerant results for the one token above the token scores.
fn add_fuzzy_matches<'a, T>(
    token: &Token,
    missed: &[Missed<'a, T>],
    scored: &mut Vec<Scored<'a, T>>,
) {
    let docs: Vec<&str> = missed.iter().map(|entry| entry.joined.as_str()).collect();
    for result in fuse::search(&docs, &token.text) {
        if let Some(entry) = missed.get(result.ref_index) {
            scored.push(Scored {
                item: entry.item,
                score: 1.0 + result.score,
                order: entry.order,
            });
        }
    }
}

/// `(a.score - b.score) || (a.order - b.order)`.
fn compare_scores<T>(left: &Scored<'_, T>, right: &Scored<'_, T>) -> Ordering {
    if left.score == right.score {
        left.order.cmp(&right.order)
    } else if left.score < right.score {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

#[cfg(test)]
#[path = "rank_test.rs"]
mod rank_test;
