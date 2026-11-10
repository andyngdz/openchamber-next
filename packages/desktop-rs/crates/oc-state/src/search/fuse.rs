//! A port of Fuse.js 7.1.0's string-list search, as `rankByQuery` uses it.
//!
//! `rankByQuery` builds `new Fuse(joinedFields, options)` and takes each result's index and
//! score (`packages/ui/src/lib/search/fuzzySearch.ts:147-156`). This module keeps that flow:
//! blank documents are dropped from the index, each remaining document gets Fuse's field-length
//! norm, the score is `score ^ norm`, and results sort by score then index.
//!
//! The norm is `1 / Math.pow(numTokens, 0.5)` rounded to three decimals (`fuse.mjs:298-301`).
//! It is ported with `sqrt`, which is correctly rounded in both languages, so the norm is
//! identical. `score ^ norm` is the one remaining `pow` call: V8 uses fdlibm and Rust the
//! platform libm, each within 1 ULP of the exact result, which is why the tests allow 2 ULP.

use std::cmp::Ordering;

use super::bitap::BitapSearch;
use super::text::is_blank;

/// `Math.pow(10, 3)`: `norm(weight, mantissa)` rounds the field-length norm to three decimals.
const NORM_SCALE: f64 = 1000.0;

/// One document Fuse matched: its index in the input and the score it reported.
pub(super) struct ScoredDoc {
    /// The document's index in the input list.
    pub(super) ref_index: usize,
    /// Fuse's score; lower is better.
    pub(super) score: f64,
}

/// Fuse's `search` over a list of strings, sorted the way Fuse sorts its results.
pub(super) fn search(docs: &[&str], query: &str) -> Vec<ScoredDoc> {
    let searcher = BitapSearch::new(query);
    let mut results: Vec<ScoredDoc> = docs
        .iter()
        .enumerate()
        .filter(|(_, doc)| !is_blank(doc))
        .filter_map(|(ref_index, doc)| {
            let norm = field_norm(doc);
            searcher.search_in(doc).map(|score| ScoredDoc {
                ref_index,
                score: compute_score(score, norm),
            })
        })
        .collect();
    results.sort_by(compare);
    results
}

/// Fuse's field-length norm: shorter fields weigh more, rounded to three decimals.
fn field_norm(text: &str) -> f64 {
    let num_tokens = text.split(' ').filter(|token| !token.is_empty()).count();
    let norm = 1.0 / (num_tokens as f64).sqrt();
    (norm * NORM_SCALE).round() / NORM_SCALE
}

/// Fuse's `computeScore` for a string list: `pow(score, norm)`, with no field weight.
fn compute_score(score: f64, norm: f64) -> f64 {
    score.powf(norm)
}

/// Fuse's default `sortFn`: ascending score, then ascending index.
fn compare(left: &ScoredDoc, right: &ScoredDoc) -> Ordering {
    if left.score == right.score {
        left.ref_index.cmp(&right.ref_index)
    } else if left.score < right.score {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

#[cfg(test)]
#[path = "fuse_test.rs"]
mod fuse_test;
