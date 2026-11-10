//! The project popup's fixed size and label fallback.

use super::{WIDTH, label};
use oc_state::Project;

#[test]
fn the_panel_is_eighteen_rem_wide() {
    assert_eq!(WIDTH, 288.);
}

#[test]
fn a_missing_label_falls_back_to_the_folder_name() {
    let project: Project =
        serde_json::from_value(serde_json::json!({"id": "p", "path": "/tmp/work/repo"}))
            .expect("project decodes");

    assert_eq!(label(&project), "repo");
}
