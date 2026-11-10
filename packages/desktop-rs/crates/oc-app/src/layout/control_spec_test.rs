use super::ControlSpec;
use crate::layout::registry::{Area, Reveal, Role};

#[test]
fn a_key_joins_region_ancestors_and_name() {
    let spec = ControlSpec::named(Area::Header, Role::Button, "Close window")
        .under(&["Main navigation", "Window controls"]);

    assert_eq!(
        spec.key,
        "header/Main navigation/Window controls/Close window"
    );
    assert_eq!(spec.name, "Close window");
}

#[test]
fn a_row_around_a_same_named_control_carries_its_role_description() {
    let row = ControlSpec::named(Area::Sidebar, Role::Button, "Projects").described("sortable");
    let toggle = ControlSpec::named(Area::Sidebar, Role::Button, "Projects");

    assert_eq!(row.key, "sidebar/Projects (sortable)");
    assert_eq!(toggle.key, "sidebar/Projects");
}

#[test]
fn a_repeated_key_is_numbered_and_the_order_of_calls_does_not_matter() {
    let spec = ControlSpec::named(Area::Sidebar, Role::Button, "New session")
        .repeated(2)
        .revealed(Reveal::Hidden)
        .under(&[]);

    assert_eq!(spec.key, "sidebar/New session#2");
    assert_eq!(spec.reveal, Reveal::Hidden);
}

#[test]
fn a_name_may_contain_a_slash() {
    let spec = ControlSpec::named(
        Area::ChatColumn,
        Role::Button,
        "Find Files scripts/perf/*.mjs",
    );

    assert_eq!(spec.key, "chatColumn/Find Files scripts/perf/*.mjs");
}
