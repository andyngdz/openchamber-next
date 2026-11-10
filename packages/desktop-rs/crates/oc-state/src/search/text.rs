//! The string primitives this search port takes from JavaScript.
//!
//! `rankByQuery` and Fuse.js measure strings in UTF-16 code units: `String.length`, `indexOf`
//! and `charAt` all count code units, so an astral character moves a position by two. The
//! functions here keep that unit so a label such as `🚀 Dự án mới` ranks as it does in React.
//!
//! Lowercasing matches too. `String.prototype.toLowerCase` and `str::to_lowercase` both apply the
//! full Unicode case mappings: per-code-point mappings agree for every scalar value under the
//! toolchains this port is checked with, and both apply the Greek final sigma rule the same way.
//! They can differ only where JavaScript holds something Rust cannot represent — a lone
//! surrogate — which JSON cannot carry into this port either, and where a later Unicode version
//! adds a mapping to one runtime before the other. The `non_ascii_lowercasing_matches_javascript`
//! test pins the mappings the pickers meet.
//!
//! JavaScript's whitespace is not Rust's: `\s` and `trim()` include U+FEFF and exclude U+0085,
//! while `char::is_whitespace` does the opposite. [`is_whitespace`] lists JavaScript's set, so a
//! query tokenizes as it does in React.

/// JavaScript's `String.prototype.toLowerCase` for a string Rust can represent.
pub(super) fn lowercase(value: &str) -> String {
    value.to_lowercase()
}

/// One code point of JavaScript's `\s` (WhiteSpace and LineTerminator).
pub(super) fn is_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// JavaScript's `String.prototype.trim`.
pub(super) fn trim(value: &str) -> &str {
    value.trim_matches(is_whitespace)
}

/// JavaScript's `!value.trim().length`, which Fuse uses to drop a blank document from its index.
pub(super) fn is_blank(value: &str) -> bool {
    value.chars().all(is_whitespace)
}

/// `value.replace(/[^a-z0-9]+/g, '')`: keeps ASCII letters and digits only, so `gpt-4o` and
/// `gpt4o` meet.
pub(super) fn compact(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

/// The UTF-16 code units of `value`, the unit JavaScript measures strings in.
pub(super) fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

/// JavaScript's `value.length` for `value`.
pub(super) fn unit_len(value: &str) -> usize {
    value.encode_utf16().count()
}

/// The first index of `needle` in `haystack` at or after `from`, in UTF-16 code units, as
/// `String.prototype.indexOf` reports it.
pub(super) fn find_from(haystack: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    if needle.is_empty() || from > haystack.len() {
        return None;
    }
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

#[cfg(test)]
#[path = "text_test.rs"]
mod text_test;
