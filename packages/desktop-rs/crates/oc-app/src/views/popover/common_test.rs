//! The popup row metrics.

use super::{PROJECT_ICON, ROW_PADDING_Y, project_row_height};
use crate::views::text_metrics::{META, UiMetrics, line_height, resolved_ui_metrics};

#[test]
fn the_project_row_is_its_padding_plus_the_inline_flex_line_box() {
    // `px-2 py-1.5 typography-meta` (`command.tsx:121-131`): vertical padding above and
    // below. The label's inline-flex (`DraftTargetSelectors.tsx:131`) rises the meta line
    // box above the baseline, and the strut's descender follows below it. The descender
    // comes from the ascent and descent GPUI read for the resolved UI font.
    let (family, metrics) = resolved_ui_metrics(META);
    let row = project_row_height(metrics);

    let label_box = line_height(META).max(PROJECT_ICON);
    let strut = (line_height(META) - metrics.ascent + metrics.descent) / 2.;
    let expected = 2. * ROW_PADDING_Y + label_box + strut;
    assert!(
        (row - expected).abs() < 1e-4,
        "{family}: the row must follow the resolved font: {row} != {expected} ({metrics:?})"
    );
}

#[test]
fn the_project_row_follows_the_metrics_not_a_pinned_font() {
    // A taller ascent shrinks the strut descender by half the difference, so the row must
    // track the metrics it is handed rather than a fixed Adwaita Sans constant.
    let base = UiMetrics {
        ascent: 10.,
        descent: 3.,
    };
    let taller = UiMetrics {
        ascent: 14.,
        descent: 3.,
    };
    let difference = project_row_height(taller) - project_row_height(base);
    assert!((difference + 2.).abs() < 1e-4, "row delta {difference}");
}
