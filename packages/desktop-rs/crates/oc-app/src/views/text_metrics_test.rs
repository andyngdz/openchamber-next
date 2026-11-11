//! The CSS box arithmetic the picker geometry is built from.

use super::{
    META, MICRO, UiMetrics, inline_flex_line_box, line_height, resolved_ui_metrics, strut_descent,
};

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
    // `py-1` 8 + `typography-meta` 18 (`dropdown-menu.styles.ts` item), the Thinking row.
    assert_eq!(8. + line_height(META), 26.);
    // `pt-1` 4 + `pb-1.5` 6 + border-t 1 + 17 (footer at `ModelControls.tsx:2772`).
    assert_eq!(4. + 6. + 1. + line_height(MICRO), 28.);
}

#[test]
fn the_strut_descender_follows_the_resolved_font_metrics() {
    let (family, metrics) = resolved_ui_metrics(META);
    // A specified CSS line-height overrides the font's natural line box, so the strut's
    // descender is `(line-height - ascent + descent) / 2` for the font GPUI resolved.
    let expected = (line_height(META) - metrics.ascent + metrics.descent) / 2.;
    assert!(
        (strut_descent(metrics, META) - expected).abs() < 1e-4,
        "{family}: the strut must follow the resolved font ({metrics:?})"
    );
    // It is derived, not pinned: a taller ascent moves the descender by half as much.
    let taller = UiMetrics {
        ascent: metrics.ascent + 4.,
        descent: metrics.descent,
    };
    assert!(
        (strut_descent(taller, META) - (expected - 2.)).abs() < 1e-4,
        "the strut follows the ascent it is given"
    );
}

#[test]
fn the_project_label_is_an_inline_flex_box_plus_the_strut() {
    let (family, metrics) = resolved_ui_metrics(META);
    // `ProjectLabel`'s inline-flex (`DraftTargetSelectors.tsx:131`) with the
    // `size-4` icon (`command.tsx:129`) in an 18px meta line.
    let label_box = line_height(META).max(16.);
    let line_box = inline_flex_line_box(metrics, label_box, META);
    let strut = (line_height(META) - metrics.ascent + metrics.descent) / 2.;
    let expected = label_box + strut;
    assert!(
        (line_box - expected).abs() < 1e-4,
        "{family}: the line box must follow the resolved font ({metrics:?})"
    );
}
