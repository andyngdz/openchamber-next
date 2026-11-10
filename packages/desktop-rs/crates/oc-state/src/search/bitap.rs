//! Fuse.js 7.1.0's Bitap search over UTF-16 code units.
//!
//! Ported from `packages/ui/node_modules/fuse.js/dist/fuse.mjs` (`BitapSearch`, `search` and
//! `computeScore`) with the options `rankByQuery` passes: threshold 0.35, `ignoreLocation`,
//! distance 100, `minMatchCharLength` 2 and `isCaseSensitive` false. With `ignoreLocation` set,
//! `computeScore` is `errors / pattern.length`, so location and distance drop out. Patterns longer
//! than 32 units are chunked the same way, and a chunk that does not match still adds its `1` to
//! the average, as it does in Fuse.

use std::collections::HashMap;

use super::mask::MatchMask;
use super::scan::{Scan, accuracy};
use super::text::{find_from, lowercase, units};

/// Fuse's `MAX_BITS`: the pattern length one Bitap run covers.
const MAX_PATTERN_UNITS: usize = 32;
/// `rankByQuery`'s `threshold`.
const THRESHOLD: f64 = 0.35;
/// `rankByQuery`'s `minMatchCharLength`.
const MIN_MATCH_CHAR_LENGTH: usize = 2;
/// Fuse's floor for a chunk score, so an exact match is still positive.
const SCORE_FLOOR: f64 = 0.001;

/// A search pattern and its chunks, built once per query.
pub(super) struct BitapSearch {
    pattern: Vec<u16>,
    chunks: Vec<Chunk>,
}

impl BitapSearch {
    /// Builds the searcher for `pattern`, lowercased as `isCaseSensitive: false` does.
    pub(super) fn new(pattern: &str) -> Self {
        let pattern = units(&lowercase(pattern));
        let chunks = if pattern.is_empty() {
            Vec::new()
        } else {
            chunk_pattern(&pattern)
        };
        Self { pattern, chunks }
    }

    /// Fuse's score for `text`, or `None` when it does not match. Lower is better.
    pub(super) fn search_in(&self, text: &str) -> Option<f64> {
        let text = units(&lowercase(text));
        if self.pattern == text {
            return Some(0.0);
        }
        if self.chunks.is_empty() {
            return None;
        }
        let mut total_score = 0.0;
        let mut has_match = false;
        for chunk in &self.chunks {
            let chunk_result = search_chunk(&text, chunk);
            has_match |= chunk_result.is_match;
            total_score += chunk_result.score;
        }
        has_match.then_some(total_score / self.chunks.len() as f64)
    }
}

/// One Bitap run: at most [`MAX_PATTERN_UNITS`] pattern units and where they start.
pub(super) struct Chunk {
    /// The pattern units this chunk covers.
    pub(super) pattern: Vec<u16>,
    /// Each pattern unit's bit mask, as Fuse's `createPatternAlphabet` builds it.
    pub(super) alphabet: HashMap<u16, i32>,
    /// Where this chunk starts in the whole pattern; Fuse adds it to `location`.
    start_index: usize,
}

impl Chunk {
    /// The chunk of `pattern` that starts at `start_index` in the whole pattern.
    fn new(pattern: &[u16], start_index: usize) -> Self {
        let mut alphabet = HashMap::new();
        for (position, unit) in pattern.iter().enumerate() {
            let mask = 1_i32 << (pattern.len() - position - 1);
            *alphabet.entry(*unit).or_insert(0) |= mask;
        }
        Self {
            pattern: pattern.to_vec(),
            alphabet,
            start_index,
        }
    }
}

/// Splits a pattern the way `BitapSearch`'s constructor does.
fn chunk_pattern(pattern: &[u16]) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    if pattern.len() > MAX_PATTERN_UNITS {
        let remainder = pattern.len() % MAX_PATTERN_UNITS;
        let mut start = 0;
        while start < pattern.len() - remainder {
            chunks.push(Chunk::new(
                &pattern[start..start + MAX_PATTERN_UNITS],
                start,
            ));
            start += MAX_PATTERN_UNITS;
        }
        if remainder > 0 {
            let start = pattern.len() - MAX_PATTERN_UNITS;
            chunks.push(Chunk::new(&pattern[start..], start));
        }
    } else {
        chunks.push(Chunk::new(pattern, 0));
    }
    chunks
}

/// One chunk's verdict: whether it matched and the score Fuse would report for it.
struct ChunkResult {
    is_match: bool,
    score: f64,
}

/// Fuse's `search`: exact matches first as a speed-up, then the Bitap scan.
fn search_chunk(text: &[u16], chunk: &Chunk) -> ChunkResult {
    let expected_location = chunk.start_index.min(text.len());
    let mut mask = MatchMask::new(text.len());
    let threshold = mark_exact_matches(text, chunk, expected_location, &mut mask);
    let scan = Scan {
        text,
        chunk,
        expected_location,
        mask: &mut mask,
        threshold,
        best_location: None,
        final_score: 1.0,
    };
    let (best_location, final_score) = scan.run();
    let is_match = best_location.is_some() && mask.has_run(MIN_MATCH_CHAR_LENGTH);
    ChunkResult {
        is_match,
        score: final_score.max(SCORE_FLOOR),
    }
}

/// Fuse's exact-match pre-scan: lowers the threshold and marks the matched units.
fn mark_exact_matches(
    text: &[u16],
    chunk: &Chunk,
    expected_location: usize,
    mask: &mut MatchMask,
) -> f64 {
    let mut threshold = THRESHOLD;
    let mut search_from = expected_location;
    while let Some(index) = find_from(text, &chunk.pattern, search_from) {
        threshold = threshold.min(accuracy(chunk.pattern.len(), 0));
        search_from = index + chunk.pattern.len();
        for offset in 0..chunk.pattern.len() {
            mask.set(index + offset, 1);
        }
    }
    threshold
}

#[cfg(test)]
#[path = "bitap_test.rs"]
mod bitap_test;
