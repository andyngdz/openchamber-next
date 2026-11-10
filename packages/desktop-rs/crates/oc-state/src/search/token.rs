//! Query tokens and item fields, and the score one token gets against one item.
//!
//! Both sides are lowercased and measured in UTF-16 code units, and both keep a
//! punctuation-stripped "compact" form so `gpt4o` finds `gpt-4o` (see [`super::text`]).

use super::text::{compact, find_from, is_whitespace, lowercase, trim, unit_len, units};

/// Added per field, so an earlier field wins a tie against a later one.
const FIELD_PENALTY: f64 = 0.01;
/// A substring that starts a word.
const WORD_BOUNDARY_SCORE: f64 = 0.1;
/// A substring inside a word.
const MID_WORD_SCORE: f64 = 0.2;
/// A punctuation-insensitive substring, behind every literal one.
const COMPACT_SCORE: f64 = 0.4;
/// Position weight: `idx / 1000` in `scoreRankToken`.
const POSITION_SCALE: f64 = 1000.0;

/// A lowercased query token with the forms every field is matched against.
pub(super) struct Token {
    /// The lowercased token, as Fuse searches it.
    pub(super) text: String,
    units: Vec<u16>,
    compact: String,
}

/// One field of an item, lowercased, with the forms a token is matched against.
pub(super) struct RankField {
    /// The lowercased field, joined with its siblings for Fuse.
    pub(super) lower: String,
    units: Vec<u16>,
    compact: String,
}

/// `query.trim().toLowerCase().split(/\s+/)`, without the empty pieces.
pub(super) fn tokenize(query: &str) -> Vec<Token> {
    let lowered = lowercase(trim(query));
    lowered
        .split(is_whitespace)
        .filter(|text| !text.is_empty())
        .map(|text| Token {
            text: text.to_owned(),
            units: units(text),
            compact: compact(text),
        })
        .collect()
}

/// Lowercased fields, skipping the empty texts JavaScript's falsy check skips.
pub(super) fn build_rank_fields(texts: &[&str]) -> Vec<RankField> {
    texts
        .iter()
        .filter(|text| !text.is_empty())
        .map(|text| {
            let lower = lowercase(text);
            RankField {
                units: units(&lower),
                compact: compact(&lower),
                lower,
            }
        })
        .collect()
}

/// `scoreRankToken`: the best score one token gets across the fields, or `None` for a miss.
pub(super) fn score_token(token: &Token, fields: &[RankField]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for (field_index, field) in fields.iter().enumerate() {
        let penalty = field_index as f64 * FIELD_PENALTY;
        let score = match find_from(&field.units, &token.units, 0) {
            Some(0) => Some(penalty),
            Some(index) => {
                let base = if is_word_separator(field.units[index - 1]) {
                    WORD_BOUNDARY_SCORE
                } else {
                    MID_WORD_SCORE
                };
                Some(base + index as f64 / POSITION_SCALE + penalty)
            }
            None => compact_score(token, field).map(|score| score + penalty),
        };
        if let Some(score) = score {
            best = Some(best.map_or(score, |current| current.min(score)));
        }
    }
    best
}

/// The compact-match score of a token in one field, or `None` when the compact form misses.
fn compact_score(token: &Token, field: &RankField) -> Option<f64> {
    if token.compact.is_empty() || unit_len(&token.text) <= 1 {
        return None;
    }
    field
        .compact
        .find(&token.compact)
        .map(|index| COMPACT_SCORE + index as f64 / POSITION_SCALE)
}

/// `!/[a-z0-9]/.test(field[idx - 1])`: the unit before the match does not continue a word.
fn is_word_separator(unit: u16) -> bool {
    !matches!(unit, 0x30..=0x39 | 0x61..=0x7A)
}

#[cfg(test)]
#[path = "token_test.rs"]
mod token_test;
