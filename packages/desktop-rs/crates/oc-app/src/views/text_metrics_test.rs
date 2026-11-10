//! The CSS box arithmetic the picker geometry is built from.

use super::{
    META, MICRO, ascent, descent, inline_flex_line_box, line_gap, line_height, strut_descent,
};

#[test]
fn the_semantic_sizes_use_their_round_up_line_heights() {
    // `--text-meta` 12.5px -> `round(1.45em, 1px)` = 18 (`typography.css:27-29`).
    assert_eq!(line_height(META), 18.);
    // `--text-micro` 12px -> 17 (`typography.css:31-34`).
    assert_eq!(line_height(MICRO), 17.);
}

#[test]
fn the_metrics_come_from_the_adwaita_sans_file() {
    assert!((ascent(META) - 12.109_375).abs() < 1e-4);
    assert!((descent(META) - 3.015_136_7).abs() < 1e-4);
    assert_eq!(line_gap(META), 0.);
    // (18 - 12.109375 + 3.0151367) / 2.
    assert!((strut_descent(META) - 4.452_881).abs() < 1e-4);
}

#[test]
fn a_row_is_its_vertical_padding_plus_one_line() {
    // `py-1.5` 12 + `typography-meta` 18 (`command.tsx:127`), the model row.
    assert_eq!(12. + line_height(META), 30.);
    // `py-1.5` 12 + `typography-micro` 17 (`MobileSheetList.tsx:11`), a section header.
    assert_eq!(12. + line_height(MICRO), 29.);
    // `py-1` 8 + `typography-meta` 18 (`dropdown-menu.styles.ts` item), the Thinking row.
    assert_eq!(8. + line_height(META), 26.);
    // `pt-1` 4 + `pb-1.5` 6 + border-t 1 + 17 (footer at `ModelControls.tsx:2772`).
    assert_eq!(4. + 6. + 1. + line_height(MICRO), 28.);
}

#[test]
fn the_project_label_is_an_inline_flex_box_plus_the_strut() {
    // `ProjectLabel`'s inline-flex (`DraftTargetSelectors.tsx:131`) with the
    // `size-4` icon (`command.tsx:129`) in an 18px meta line.
    let line_box = inline_flex_line_box(line_height(META).max(16.), META);
    assert!((line_box - 22.452_88).abs() < 1e-4);
    assert!((12. + line_box - 34.452_88).abs() < 1e-4);
}
