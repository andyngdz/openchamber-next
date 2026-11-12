//! The shared picker row helpers.

use super::{Row, RowAction};

#[test]
fn a_row_keeps_its_accessible_name_apart_from_what_it_draws() {
    let row = Row {
        name: "Fixture model 200K".to_owned(),
        label: "Fixture model".to_owned(),
        icon: None,
        logo: None,
        dot: None,
        context: Some("200K".to_owned()),
        selected: false,
        role: crate::layout::Role::Option,
        ancestors: vec!["Models".to_owned()],
        action: RowAction::Close,
        star: None,
        described: None,
        repeat: None,
        star_under_row: true,
    };
    assert_eq!(row.name, "Fixture model 200K");
    assert_eq!(row.label, "Fixture model");
    assert_eq!(row.context.as_deref(), Some("200K"));
    assert!(matches!(row.action, RowAction::Close));
}
