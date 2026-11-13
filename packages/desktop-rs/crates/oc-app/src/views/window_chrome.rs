//! What a client-decorated window draws for itself: the 44 px drag strip every zone starts
//! with, and the window controls over the right-hand zone.

use super::header::MAIN_NAVIGATION;
use super::{Scene, icon_button};
use crate::icons::RemixIcon;
use crate::layout::{Area, ControlSpec, Role, control};
use gpui_kit::{
    AnyElement, App, Div, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, Styled,
    Window, div, px,
};

/// Height of the strip along the top of each zone, which holds its header and moves the window.
pub(crate) const STRIP_HEIGHT: f32 = 44.;
/// Width the window controls take at the top right; the right-hand header ends before it.
pub(crate) const CONTROLS_WIDTH: f32 = 120.;

/// Makes `zone` move the window when pressed on its empty area, and toggle maximize on a
/// double click.
///
/// GPUI reports a press to every hovered element, children first, so a button inside `zone`
/// is told too. A button marks its press as handled with `prevent_default`; `zone` then
/// leaves it alone, so pressing Minimize or a header tool never starts a move.
pub(crate) fn drag_region(zone: Div) -> Div {
    zone.on_mouse_down(MouseButton::Left, drag_or_maximize)
}

fn drag_or_maximize(event: &MouseDownEvent, window: &mut Window, _: &mut App) {
    if window.default_prevented() {
        return;
    }
    if event.click_count >= 2 {
        window.zoom_window();
    } else {
        window.start_window_move();
    }
}

/// The empty drag strip on top of the right-hand zone. It spans the zone's width; the window
/// controls draw over its right end.
pub(crate) fn panel_drag_strip() -> Div {
    drag_region(
        div()
            .h(px(STRIP_HEIGHT))
            .w_full()
            .flex_none()
            .pr(px(CONTROLS_WIDTH)),
    )
}

/// What a window control does when clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowAction {
    Minimize,
    Maximize,
    Close,
}

/// The reference's group name of the three window controls.
const WINDOW_CONTROL_GROUP: &str = "Window controls";

/// One 34x28 ghost window control; its name and place in the order come from the reference.
pub(super) fn window_control(
    scene: Scene<'_>,
    name: &'static str,
    icon: RemixIcon,
    action: WindowAction,
) -> AnyElement {
    let registry = scene.registry;
    let spec = ControlSpec::named(Area::Header, Role::Button, name)
        .under(&[MAIN_NAVIGATION, WINDOW_CONTROL_GROUP]);
    let button = icon_button(registry, &spec, icon, scene.palette.muted_foreground.into())
        .w(px(34.))
        .h(px(28.))
        .on_click(move |_, window, _| match action {
            WindowAction::Minimize => window.minimize_window(),
            WindowAction::Maximize => window.zoom_window(),
            WindowAction::Close => window.remove_window(),
        });
    control(registry, spec, button).into_any_element()
}

#[cfg(test)]
#[path = "window_chrome_test.rs"]
mod window_chrome_test;
