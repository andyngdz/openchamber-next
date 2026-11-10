//! The JavaScript string primitives the search port relies on.

use super::{compact, find_from, is_blank, is_whitespace, lowercase, trim, unit_len, units};

#[test]
fn non_ascii_lowercasing_matches_javascript() {
    // Expected values from Node 26: `String.prototype.toLowerCase`.
    assert_eq!(lowercase("DỰ ÁN OpenChamber"), "dự án openchamber");
    assert_eq!(lowercase("İSTANBUL"), "i\u{307}stanbul");
    assert_eq!(lowercase("ẞ"), "ß");
    assert_eq!(lowercase("Ⅷ"), "ⅷ");
    // The Greek final sigma, which both languages apply per word.
    assert_eq!(lowercase("ΟΔΟΣ"), "οδο\u{3C2}");
    // An astral case pair: U+10400 DESERET CAPITAL LETTER LONG I comes down to U+10428.
    assert_eq!(lowercase("𐐀𐐨"), "𐐨𐐨");
}

#[test]
fn whitespace_is_javascripts_set_not_rusts() {
    assert!(is_whitespace('\u{FEFF}'));
    assert!(!is_whitespace('\u{0085}'));
    assert_eq!(trim("\u{FEFF} chat \u{3000}"), "chat");
    assert_eq!(trim("\u{0085}chat"), "\u{0085}chat");
    assert!(is_blank(" \u{3000}\u{FEFF}"));
    assert!(!is_blank("\u{0085}"));
}

#[test]
fn compact_keeps_ascii_letters_and_digits_only() {
    assert_eq!(compact("gpt-4o-mini"), "gpt4omini");
    assert_eq!(compact("..."), "");
    assert_eq!(compact("Dự-án_1"), "Dn1");
}

#[test]
fn positions_count_utf16_code_units() {
    assert_eq!(unit_len("🚀"), 2);
    assert_eq!(unit_len("Dự"), 2);
    assert_eq!(units("🚀"), vec![0xD83D, 0xDE80]);

    let haystack = units("a🚀b");
    assert_eq!(find_from(&haystack, &units("🚀"), 0), Some(1));
    assert_eq!(find_from(&haystack, &units("b"), 0), Some(3));
    assert_eq!(find_from(&haystack, &units("b"), 4), None);
    assert_eq!(find_from(&haystack, &units("🚀b"), 2), None);
}
