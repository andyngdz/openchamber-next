//! One Bitap scan of Fuse.js 7.1.0: the bit arrays of every error level of one chunk.
//!
//! Every step is the double arithmetic of `fuse.mjs`'s `search`, with the bit arrays kept as
//! 32-bit signed integers because JavaScript's `<<`, `|` and `&` convert to `int32`. The array
//! slots Fuse leaves sparse read as `0`, as `undefined` does in a bitwise expression.

use super::bitap::Chunk;
use super::mask::MatchMask;

/// `computeScore` with `ignoreLocation`: the share of the pattern allowed to be wrong.
pub(super) fn accuracy(pattern_len: usize, errors: usize) -> f64 {
    errors as f64 / pattern_len as f64
}

/// The Bitap scan state of one chunk across its error levels.
pub(super) struct Scan<'a> {
    pub(super) text: &'a [u16],
    pub(super) chunk: &'a Chunk,
    pub(super) expected_location: usize,
    pub(super) mask: &'a mut MatchMask,
    pub(super) threshold: f64,
    pub(super) best_location: Option<usize>,
    pub(super) final_score: f64,
}

impl Scan<'_> {
    /// Runs one error level after another until Fuse gives up.
    pub(super) fn run(mut self) -> (Option<usize>, f64) {
        let pattern_len = self.chunk.pattern.len();
        let mut bin_max = self.text.len() + pattern_len;
        let mut last_bit_arr: Vec<i32> = Vec::new();
        for errors in 0..pattern_len {
            bin_max = self.binary_search_bin_max(errors, bin_max);
            let bit_arr = self.scan_level(errors, bin_max, &last_bit_arr);
            if accuracy(pattern_len, errors + 1) > self.threshold {
                break;
            }
            last_bit_arr = bit_arr;
        }
        (self.best_location, self.final_score)
    }

    /// Fuse's binary search for how far from the match location this error level may stray.
    fn binary_search_bin_max(&self, errors: usize, mut bin_max: usize) -> usize {
        let mut bin_min = 0;
        let mut bin_mid = bin_max;
        while bin_min < bin_mid {
            if accuracy(self.chunk.pattern.len(), errors) <= self.threshold {
                bin_min = bin_mid;
            } else {
                bin_max = bin_mid;
            }
            bin_mid = (bin_max - bin_min) / 2 + bin_min;
        }
        bin_mid
    }

    /// One error level of the scan; returns its bit array for the next level.
    fn scan_level(&mut self, errors: usize, bin_max: usize, last_bit_arr: &[i32]) -> Vec<i32> {
        let pattern_len = self.chunk.pattern.len();
        let mut start = 1.max(self.expected_location.saturating_sub(bin_max) + 1);
        let finish = (self.expected_location + bin_max).min(self.text.len()) + pattern_len;
        let mut bit_arr = vec![0_i32; finish + 2];
        bit_arr[finish + 1] = (1_i32 << errors).wrapping_sub(1);
        let score_mask = 1_i32 << (pattern_len - 1);
        let mut position = finish;
        loop {
            if position < start {
                break;
            }
            let current_location = position - 1;
            let char_match = self.char_match(current_location);
            self.mask.set(current_location, u8::from(char_match != 0));
            advance_bits(&mut bit_arr, last_bit_arr, position, char_match, errors);
            if (bit_arr[position] & score_mask) != 0 {
                let score = accuracy(pattern_len, errors);
                if score <= self.threshold {
                    self.threshold = score;
                    self.best_location = Some(current_location);
                    self.final_score = score;
                    if current_location <= self.expected_location {
                        break;
                    }
                    start = 1.max((2 * self.expected_location).saturating_sub(current_location));
                }
            }
            position -= 1;
        }
        bit_arr
    }

    /// `patternAlphabet[text.charAt(location)]`: a missing unit is Fuse's `undefined`, i.e. `0`.
    fn char_match(&self, location: usize) -> i32 {
        self.text
            .get(location)
            .and_then(|unit| self.chunk.alphabet.get(unit))
            .copied()
            .unwrap_or(0)
    }
}

/// One position of Fuse's bit array: the exact match and the fuzzy carries of earlier levels.
fn advance_bits(
    bit_arr: &mut [i32],
    last_bit_arr: &[i32],
    position: usize,
    char_match: i32,
    errors: usize,
) {
    bit_arr[position] = ((bit_arr[position + 1] << 1) | 1) & char_match;
    if errors > 0 {
        let next = previous_bits(last_bit_arr, position + 1);
        let carry = previous_bits(last_bit_arr, position) | next;
        bit_arr[position] |= (carry << 1) | 1 | next;
    }
}

/// A bit array entry of the previous error level; a sparse JavaScript slot reads as `0`.
fn previous_bits(bit_arr: &[i32], position: usize) -> i32 {
    bit_arr.get(position).copied().unwrap_or(0)
}

#[cfg(test)]
#[path = "scan_test.rs"]
mod scan_test;
