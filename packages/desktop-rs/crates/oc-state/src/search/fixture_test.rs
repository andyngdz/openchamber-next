//! Fixture files and the score tolerance shared by the search tests.

use serde::de::DeserializeOwned;

const FIXTURES: &str = env!("OC_DESKTOP_FIXTURES");

/// The 2-ULP bound design.md derives: every step of the score is IEEE-754 double arithmetic
/// Rust reproduces exactly except `score ^ norm`, where V8's `pow` (fdlibm) and Rust's `powf`
/// (the platform libm) are each within 1 ULP of the exact result.
const SCORE_ULPS: f64 = 2.0;

/// Reads one fixture of `fixtures/search/`.
pub(super) fn read<T: DeserializeOwned>(name: &str) -> T {
    let path = format!("{FIXTURES}/{name}");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// Whether two scores are within the one-`pow` bound: `|a - b| <= 2 * EPSILON * max(|a|, |b|)`.
pub(super) fn scores_match(left: f64, right: f64) -> bool {
    (left - right).abs() <= SCORE_ULPS * f64::EPSILON * left.abs().max(right.abs())
}
