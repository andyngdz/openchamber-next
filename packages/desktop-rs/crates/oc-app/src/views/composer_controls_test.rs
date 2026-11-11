//! The footer trigger's CSS box, and its labels drawn whole.

use gpui_kit::{Bounds, Pixels, TestAppContext};

use super::super::slots::{Tool, TriggerKind, toolbar};
use super::{LABEL_WEIGHT, MODEL_LABEL_MAX, TRIGGER_HEIGHT, content_selector, label_selector};
use crate::fixture::StateScript;
use crate::fixture::fixture_test::state_path;
use crate::layout::LayoutRegistry;
use crate::root::RootView;
use crate::views::text_metrics;

#[test]
fn the_trigger_height_and_gaps_are_the_css_values() {
    // `h-8` (`ModelControls.tsx:677`), `gap-1.5` model/agent (`2358,2855`), `gap-1`
    // Thinking (`2739`), `gap-x-3` footer group (`ComposerFooter.tsx:284-288`).
    assert_eq!(TRIGGER_HEIGHT, 32.);
    assert_eq!(super::GAP_MODEL, 6.);
    assert_eq!(super::GAP_THINKING, 4.);
    assert_eq!(super::GAP_AGENT, 6.);
    assert_eq!(super::GAP_GROUP, 12.);
}

/// Layout snaps each box to the device grid, so two edges may differ by one device pixel.
fn contains(outer: Bounds<Pixels>, inner: Bounds<Pixels>, device_pixel: f32) -> bool {
    f32::from(inner.left()) >= f32::from(outer.left()) - device_pixel
        && f32::from(inner.right()) <= f32::from(outer.right()) + device_pixel
}

/// Every trigger label box is at least its shaped line, the model label's up to its cap, so
/// `truncate` never ellipsizes it, and sits
/// inside the row the button leaves for its content. A padded button narrowed that row, and
/// the label lost its tail ("Buil") or shrank to an ellipsis ("defa..").
#[gpui_kit::test]
fn every_trigger_label_is_drawn_whole(cx: &mut TestAppContext) {
    let state = StateScript::load(&state_path("empty"))
        .expect("state fixture loads")
        .state();
    let triggers: Vec<(TriggerKind, String)> = toolbar(&state)
        .into_iter()
        .filter_map(|tool| match tool {
            Tool::Trigger { layout, label, .. } => Some((layout, label)),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggers.len(),
        3,
        "the fixture shows model, Thinking and agent"
    );
    cx.update(gpui_kit::init);
    let (_, cx) = cx.add_window_view(move |window, cx| {
        RootView::new(state, None, LayoutRegistry::default(), None, window, cx)
    });
    let device_pixel = cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        1. / window.scale_factor()
    });

    for (layout, label) in triggers {
        let shaped = cx.update(|window, cx| {
            crate::views::line_width(window, cx, &label, text_metrics::META, LABEL_WEIGHT)
        });
        let wanted = match layout {
            TriggerKind::Model => shaped.min(MODEL_LABEL_MAX),
            TriggerKind::Thinking | TriggerKind::Agent => shaped,
        };
        let row = cx
            .debug_bounds(content_selector(layout))
            .unwrap_or_else(|| panic!("{layout:?} trigger row was not drawn"));
        let drawn = cx
            .debug_bounds(label_selector(layout))
            .unwrap_or_else(|| panic!("{layout:?} label was not drawn"));
        assert!(
            f32::from(drawn.size.width) >= wanted,
            "{layout:?} label {label:?} drawn {:?} wide, needs {wanted}",
            drawn.size.width
        );
        assert!(
            contains(row, drawn, device_pixel),
            "{layout:?} label {label:?} at {drawn:?} runs past its row {row:?}"
        );
    }
}
