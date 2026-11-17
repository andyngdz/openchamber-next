//! The project popup's size, row height and label fallback.

use super::{WIDTH, label, project_row_height};
use crate::views::text_metrics::{META, resolved_ui_metrics, strut_descent};
use oc_state::Project;

#[test]
fn the_panel_is_eighteen_rem_wide() {
    assert_eq!(WIDTH, 288.);
}

#[test]
fn the_project_row_is_its_padding_plus_the_inline_flex_line_box() {
    // `px-2 py-1.5 typography-meta` (`command.tsx:121-131`): 6px above and below. The
    // label's inline-flex (`DraftTargetSelectors.tsx:131`) is 18px high and its baseline is
    // synthesized from the box bottom; the meta strut's descender follows.
    let (family, metrics) = resolved_ui_metrics(META);
    let row = project_row_height(metrics);

    if family == "Adwaita Sans" {
        assert!((row - 34.452_88).abs() < 1e-4, "{family}: {row}");
    } else {
        let expected = 12. + 18. + strut_descent(metrics, META);
        assert!((row - expected).abs() < 1e-4, "{family}: {row}");
    }
}

#[test]
fn a_missing_label_falls_back_to_the_folder_name() {
    let project: Project =
        serde_json::from_value(serde_json::json!({"id": "p", "path": "/tmp/work/repo"}))
            .expect("project decodes");

    assert_eq!(label(&project), "repo");
}
