//! Popup interaction through real GPUI dispatch, for every popup kind.

use gpui_kit::{
    AppContext as _, Bounds, Entity, Focusable as _, Modifiers, MouseButton, MouseDownEvent,
    Pixels, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px,
};

use super::RootView;
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::layout::{Area, LayoutRegistry};
use crate::views::PopoverKind;

/// One popup kind, its fixture and where its trigger sits. A new kind adds one row.
struct KindCase {
    kind: PopoverKind,
    fixture: &'static str,
}

const KINDS: [KindCase; 4] = [
    KindCase {
        kind: PopoverKind::Project,
        fixture: "project-popover",
    },
    KindCase {
        kind: PopoverKind::Model,
        fixture: "model-popover",
    },
    KindCase {
        kind: PopoverKind::Agent,
        fixture: "agent-menu",
    },
    KindCase {
        kind: PopoverKind::Thinking,
        fixture: "thinking-menu",
    },
];

fn open<'a>(
    cx: &'a mut TestAppContext,
    fixture: &str,
) -> (Entity<RootView>, &'a mut VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::theme::tokens(crate::theme::Mode::Dark).apply(cx);
    });
    let (state, _) = StateScript::load(&state_path(fixture))
        .expect("fixture loads")
        .into_parts();
    cx.add_window_view(move |window, cx| {
        RootView::new(state, None, LayoutRegistry::default(), None, window, cx)
    })
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.draw(cx).clear(cx);
    });
}

fn popup_open(root: &Entity<RootView>, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| root.read(cx).popover.is_some())
}

fn trigger_bounds(
    root: &Entity<RootView>,
    cx: &mut VisualTestContext,
    kind: PopoverKind,
) -> Bounds<Pixels> {
    cx.update(|_, cx| {
        let root = root.read(cx);
        let spec = crate::views::trigger_spec(kind, root.state.read(cx));
        root.registry
            .controls_in(spec.area)
            .into_iter()
            .find(|placed| placed.spec.key == spec.key)
            .unwrap_or_else(|| panic!("{kind:?} trigger is not recorded"))
            .bounds
    })
}

fn open_from_trigger(root: &Entity<RootView>, cx: &mut VisualTestContext, kind: PopoverKind) {
    draw(cx);
    let trigger = trigger_bounds(root, cx, kind);
    cx.simulate_click(trigger.center(), Modifiers::default());
    draw(cx);
    assert!(
        popup_open(root, cx),
        "{kind:?} did not open from its trigger"
    );
}

#[gpui_kit::test]
fn every_popup_closes_on_an_outside_press(cx: &mut TestAppContext) {
    for case in &KINDS {
        let (root, cx) = open(cx, case.fixture);
        open_from_trigger(&root, cx, case.kind);
        cx.simulate_click(point(px(8.), px(880.)), Modifiers::default());
        draw(cx);
        assert!(
            !popup_open(&root, cx),
            "{:?} ignored the outside press",
            case.kind
        );
    }
}

/// A press on the empty drag strip starts a window move, which the test platform answers with
/// a panic. The popup's dismissal runs before the strip's handler, so it is read after the
/// panic is caught.
#[gpui_kit::test]
fn a_press_on_the_drag_strip_still_closes_an_open_popup(cx: &mut TestAppContext) {
    for case in &KINDS {
        let (root, cx) = open(cx, case.fixture);
        open_from_trigger(&root, cx, case.kind);
        let strip = point(px(230.), px(22.));
        let moved = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cx.simulate_event(MouseDownEvent {
                position: strip,
                modifiers: Modifiers::default(),
                button: MouseButton::Left,
                click_count: 1,
                first_mouse: false,
            });
        }));
        let payload = moved.expect_err("the strip did not start a window move");
        let message = payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        assert!(
            message.contains("not implemented"),
            "{:?}: panicked with {message:?}, not the platform's move request",
            case.kind
        );
        let open = cx.read_entity(&root, |root, _| root.popover.is_some());
        assert!(
            !open,
            "{:?} stayed open under a press on the drag strip",
            case.kind
        );
    }
}

#[gpui_kit::test]
fn every_popup_closes_on_escape_and_returns_focus_to_the_composer(cx: &mut TestAppContext) {
    for case in &KINDS {
        let (root, cx) = open(cx, case.fixture);
        open_from_trigger(&root, cx, case.kind);
        cx.simulate_keystrokes("escape");
        draw(cx);
        assert!(!popup_open(&root, cx), "{:?} ignored Escape", case.kind);
        cx.update(|window, cx| {
            let draft = root.read(cx).draft.read(cx).focus_handle(cx);
            assert!(
                draft.is_focused(window),
                "{:?} did not return focus to the composer",
                case.kind
            );
        });
    }
}

#[gpui_kit::test]
fn a_wheel_over_every_popup_leaves_the_chat_column_still(cx: &mut TestAppContext) {
    for case in &KINDS {
        let (root, cx) = open(cx, case.fixture);
        open_from_trigger(&root, cx, case.kind);
        let popup = cx.update(|_, cx| {
            root.read(cx)
                .registry
                .region(Area::Popover)
                .expect("the popup is placed")
        });
        let before = cx.update(|_, cx| {
            root.update(cx, |root, _| {
                root.scroll.set_offset(point(px(0.), px(-80.)))
            });
            root.read(cx).scroll.offset()
        });
        let position = point(popup.origin.x + px(10.), popup.origin.y + px(60.));
        cx.simulate_event(ScrollEvent {
            position,
            delta: ScrollDelta::Pixels(point(px(0.), px(-120.))),
            touch_phase: gpui_kit::TouchPhase::Moved,
            ..Default::default()
        });
        draw(cx);
        let after = cx.update(|_, cx| root.read(cx).scroll.offset());
        assert_eq!(
            after, before,
            "{:?} let the wheel through to the chat column",
            case.kind
        );
    }
}

#[gpui_kit::test]
fn enter_picks_the_highlighted_row_in_every_popup(cx: &mut TestAppContext) {
    for case in &KINDS {
        let (root, cx) = open(cx, case.fixture);
        open_from_trigger(&root, cx, case.kind);
        cx.simulate_keystrokes("down enter");
        draw(cx);
        assert!(
            !popup_open(&root, cx),
            "{:?} did not pick its highlighted row on Enter",
            case.kind
        );
    }
}

#[gpui_kit::test]
fn every_popup_draws_a_placed_panel(cx: &mut TestAppContext) {
    for case in &KINDS {
        let (root, cx) = open(cx, case.fixture);
        open_from_trigger(&root, cx, case.kind);
        let placed = cx.update(|_, cx| root.read(cx).registry.region(Area::Popover).is_some());
        assert!(placed, "{:?} drew no popup panel", case.kind);
    }
}
