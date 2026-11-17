//! The window-move strips through real GPUI dispatch. The test platform panics on a window
//! move or zoom request ("not implemented"), so a press that reaches a drag handler fails
//! the test, and a press that should reach one is asserted with `should_panic`.

use gpui_kit::{
    Entity, Modifiers, MouseButton, MouseDownEvent, Pixels, Point, TestAppContext,
    VisualTestContext, point, px, size,
};

use super::RootView;
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::layout::LayoutRegistry;

fn open(cx: &mut TestAppContext) -> (Entity<RootView>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::theme::tokens(crate::theme::Mode::Dark).apply(cx);
    });
    let (state, _) = StateScript::load(&state_path("session"))
        .expect("fixture loads")
        .into_parts();
    let (root, cx) = cx.add_window_view(move |window, cx| {
        RootView::new(state, None, LayoutRegistry::default(), None, window, cx)
    });
    cx.simulate_resize(size(px(1440.), px(900.)));
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.draw(cx).clear(cx);
    });
    (root, cx)
}

fn press(cx: &mut VisualTestContext, position: Point<Pixels>, click_count: usize) {
    cx.simulate_event(MouseDownEvent {
        position,
        modifiers: Modifiers::default(),
        button: MouseButton::Left,
        click_count,
        first_mouse: false,
    });
}

/// The Maximize window control, centered (1362..1396 by 8..36 at 1440 wide).
fn maximize() -> Point<Pixels> {
    point(px(1379.), px(22.))
}

/// The sidebar's menu button, the first control in the sidebar strip.
fn sidebar_button() -> Point<Pixels> {
    point(px(20.), px(22.))
}

#[gpui_kit::test]
fn pressing_a_window_control_does_not_move_the_window(cx: &mut TestAppContext) {
    let (_root, cx) = open(cx);

    press(cx, maximize(), 1);
}

#[gpui_kit::test]
fn double_pressing_a_window_control_does_not_zoom_twice(cx: &mut TestAppContext) {
    let (_root, cx) = open(cx);

    press(cx, maximize(), 2);
}

#[gpui_kit::test]
fn pressing_a_strip_button_does_not_move_the_window(cx: &mut TestAppContext) {
    let (_root, cx) = open(cx);

    press(cx, sidebar_button(), 1);
}

#[gpui_kit::test]
#[should_panic(expected = "not implemented")]
fn pressing_the_empty_strip_moves_the_window(cx: &mut TestAppContext) {
    let (_root, cx) = open(cx);

    press(cx, point(px(230.), px(22.)), 1);
}
