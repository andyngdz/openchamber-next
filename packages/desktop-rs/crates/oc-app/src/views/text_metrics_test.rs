//! The CSS box arithmetic the picker rows are built from.

use super::{META, MICRO, line_height};

#[test]
fn the_semantic_sizes_use_their_round_up_line_heights() {
    // `--text-meta` 12.5px -> `round(1.45em, 1px)` = 18 (`typography.css:27-29`).
    assert_eq!(line_height(META), 18.);
    // `--text-micro` 12px -> 17 (`typography.css:31-34`).
    assert_eq!(line_height(MICRO), 17.);
}

#[test]
fn a_row_is_its_vertical_padding_plus_one_line() {
    // `py-1.5` 12 + `typography-meta` 18 (`command.tsx:127`), the model row.
    assert_eq!(12. + line_height(META), 30.);
    // `py-1.5` 12 + `typography-micro` 17 (`MobileSheetList.tsx:11`), a section header.
    assert_eq!(12. + line_height(MICRO), 29.);
    // `px-2 py-1` 8 + `typography-meta` 18 (`dropdown-menu.styles.ts` item).
    assert_eq!(8. + line_height(META), 26.);
}
