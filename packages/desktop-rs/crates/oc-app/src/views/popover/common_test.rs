//! The popup row metrics.

use super::project_row_height;

#[test]
fn the_project_row_is_its_padding_plus_the_inline_flex_line_box() {
    // `px-2 py-1.5 typography-meta` (`command.tsx:121-131`): 6px above and below.
    // The label's inline-flex (`DraftTargetSelectors.tsx:131`) is 18px high and its
    // baseline is synthesized from the box bottom; the meta strut's descender adds
    // 4.452881.
    assert!((project_row_height() - 34.452_88).abs() < 1e-4);
}
