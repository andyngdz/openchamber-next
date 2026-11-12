use gpui_kit::{Bounds, point, px, size};
use serde_json::Value;

use super::LayoutDump;
use crate::errors::AppError;
use crate::layout::registry::{Area, ControlSpec, LayoutRegistry, Reveal, Role};

const ALL_AREAS: [Area; 6] = [
    Area::Sidebar,
    Area::Header,
    Area::ChatColumn,
    Area::Composer,
    Area::WorkStatusPanel,
    Area::IconRail,
];

fn filled_registry() -> LayoutRegistry {
    let registry = LayoutRegistry::default();
    registry.record_content(Bounds::new(
        point(px(21.), px(21.)),
        size(px(1440.), px(900.)),
    ));
    for area in ALL_AREAS {
        registry.record_region(
            area,
            Bounds::new(point(px(301.), px(69.)), size(px(792.), px(852.))),
        );
    }
    registry
}

#[test]
fn dump_has_the_reference_shape() {
    let registry = filled_registry();
    registry.record_control(
        ControlSpec::named(Area::Sidebar, Role::Button, "Archive")
            .under(&["Projects", "Reference: finished turn"])
            .revealed(Reveal::OnHover("row".into())),
        Bounds::default(),
    );

    let dump = LayoutDump::capture("session", 1.25, &registry)
        .and_then(|dump| dump.to_json())
        .expect("dump encodes");
    let json: Value = serde_json::from_str(&dump).expect("dump is JSON");

    assert_eq!(json["state"], "session");
    assert_eq!(json["viewport"]["width"], 1440.0);
    assert_eq!(json["scaleFactor"], 1.25);
    assert_eq!(json["regions"]["chatColumn"]["bounds"]["x"], 280.0);
    assert_eq!(json["regions"]["chatColumn"]["bounds"]["y"], 48.0);
    assert_eq!(
        json["regions"]["sidebar"]["controls"][0],
        serde_json::json!({
            "role": "button",
            "name": "Archive",
            "key": "sidebar/Projects/Reference: finished turn/Archive",
            "shown": false,
            "bounds": {"x": -21.0, "y": -21.0, "width": 0.0, "height": 0.0},
            "icons": [],
            "marks": [],
            "texts": []
        })
    );
    assert!(dump.ends_with('\n'));
}

#[test]
fn bounds_are_rounded_to_hundredths() {
    let registry = filled_registry();
    registry.record_region(
        Area::Header,
        Bounds::new(point(px(21.), px(69.000_01)), size(px(315.2), px(48.))),
    );

    let dump = LayoutDump::capture("empty", 1.25, &registry)
        .and_then(|dump| dump.to_json())
        .expect("dump encodes");
    let json: Value = serde_json::from_str(&dump).expect("dump is JSON");

    assert_eq!(json["regions"]["header"]["bounds"]["y"], 48.0);
    assert_eq!(json["regions"]["header"]["bounds"]["width"], 315.2);
}

#[test]
fn a_frame_without_content_is_an_error() {
    let error =
        LayoutDump::capture("empty", 1.25, &LayoutRegistry::default()).expect_err("no content");

    assert!(matches!(error, AppError::RegionMissing("content")));
}

#[test]
fn a_region_that_was_not_laid_out_is_an_error() {
    let registry = LayoutRegistry::default();
    registry.record_content(Bounds::default());
    registry.record_region(Area::Sidebar, Bounds::default());

    let error = LayoutDump::capture("empty", 1.25, &registry).expect_err("header is missing");

    assert!(matches!(error, AppError::RegionMissing("header")));
}
