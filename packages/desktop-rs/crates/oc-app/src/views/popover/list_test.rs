//! The shared picker row helpers.

use super::{Row, RowAction, Star, StarKind};

#[test]
fn a_row_keeps_its_accessible_name_and_visible_text() {
    let row = Row {
        name: "Fixture model".to_owned(),
        visible: "Fixture model 200K".to_owned(),
        role: crate::layout::Role::Option,
        ancestors: vec!["Models".to_owned()],
        action: RowAction::Close,
        star: None,
        described: None,
        repeat: None,
        star_under_row: true,
    };
    assert_eq!(row.name, "Fixture model");
    assert_eq!(row.visible, "Fixture model 200K");
    assert!(matches!(row.action, RowAction::Close));
}

#[test]
fn the_star_kind_names_the_icon() {
    let starred = Star {
        label: "Unfavorite",
        action: oc_state::SelectionAction::ToggleFavoriteAgent("build".to_owned()),
        kind: StarKind::Filled,
        shown: true,
    };
    assert!(matches!(starred.kind, StarKind::Filled));
}
