//! The model search field's text handling.

use super::trimmed;

#[test]
fn the_filter_ignores_surrounding_whitespace() {
    assert_eq!(trimmed("  fixture  "), "fixture");
    assert_eq!(trimmed(""), "");
}
