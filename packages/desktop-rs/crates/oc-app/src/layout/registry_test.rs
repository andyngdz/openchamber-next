use gpui_kit::{Bounds, point, px, size};

use super::{Area, ControlSpec, LayoutRegistry, Role};

fn square(origin: f32) -> Bounds<gpui_kit::Pixels> {
    Bounds::new(point(px(origin), px(origin)), size(px(10.), px(10.)))
}

#[test]
fn controls_keep_prepaint_order_per_region() {
    let registry = LayoutRegistry::default();
    registry.record_control(
        ControlSpec::named(Area::Sidebar, Role::Button, "Chats"),
        square(1.),
    );
    registry.record_control(
        ControlSpec::named(Area::Header, Role::Button, "Close window"),
        square(2.),
    );
    registry.record_control(
        ControlSpec::named(Area::Sidebar, Role::Button, "Settings"),
        square(3.),
    );

    let keys: Vec<_> = registry
        .controls_in(Area::Sidebar)
        .into_iter()
        .map(|placed| placed.spec.key)
        .collect();
    assert_eq!(keys, ["sidebar/Chats", "sidebar/Settings"]);
}

#[test]
fn begin_frame_forgets_the_previous_frame() {
    let registry = LayoutRegistry::default();
    registry.record_region(Area::Composer, square(4.));
    registry.record_control(
        ControlSpec::named(Area::Composer, Role::Button, "Send message"),
        square(5.),
    );

    registry.begin_frame();

    assert_eq!(registry.region(Area::Composer), None);
    assert!(registry.controls_in(Area::Composer).is_empty());
}
