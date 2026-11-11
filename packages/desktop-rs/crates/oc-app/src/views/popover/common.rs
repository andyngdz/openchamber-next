//! Shared popup row and panel helpers.

use gpui_kit::SharedString;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    App, Div, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, px,
};

use crate::layout::{Area, ControlSpec, Role, control as control_element, region};
use crate::views::Intent;
use crate::views::Scene;
use crate::views::popover::Pick;
use crate::views::text_metrics;

/// `px-2` (`command.tsx:127`): 0.5rem at the 16px root.
const ROW_PADDING_X: f32 = 8.;
/// `py-1.5` (`command.tsx:127`): 0.375rem.
const ROW_PADDING_Y: f32 = 6.;
/// `rounded-lg` (`command.tsx:127`) = `--radius-lg` 0.5625rem (`design-system.css:893`).
const ROW_RADIUS: f32 = 9.;
/// `size-4` on the project icon (`command.tsx:129`): 1rem.
const PROJECT_ICON: f32 = 16.;
/// `rounded-xl` (`dropdown-menu.styles.ts:1`) = `--radius-xl` 0.75rem
/// (`design-system.css:894`).
const PANEL_RADIUS: f32 = 12.;

/// A control inside the popup, keyed by its accessibility path.
pub(super) fn control(
    scene: Scene<'_>,
    role: Role,
    name: impl Into<SharedString>,
    ancestors: &[&str],
    content: impl IntoElement,
) -> Div {
    let spec = ControlSpec::named(Area::Popover, role, name).under(ancestors);
    control_element(scene.registry, spec, content)
}

/// A control laid out but transparent until its row is hovered, like the star toggles.
pub(super) fn hidden_control(
    scene: Scene<'_>,
    role: Role,
    name: impl Into<SharedString>,
    ancestors: &[&str],
    content: impl IntoElement,
) -> Div {
    let spec = ControlSpec::named(Area::Popover, role, name)
        .under(ancestors)
        .revealed(crate::layout::Reveal::Hidden);
    control_element(scene.registry, spec, content)
}

/// One clickable popup row with the standard row metrics (`px-2 py-1.5 typography-meta`).
pub(super) fn option_row(
    scene: Scene<'_>,
    cx: &App,
    role: Role,
    name: impl Into<SharedString>,
    ancestors: &[&str],
    pick: Pick,
) -> Div {
    let name: SharedString = name.into();
    let row = div()
        .id(name.clone())
        .h(px(project_row_height(text_metrics::UiMetrics::ui(
            cx,
            text_metrics::META,
        ))))
        .px(px(ROW_PADDING_X))
        .rounded(px(ROW_RADIUS))
        .flex()
        .items_center()
        .text_size(px(text_metrics::META))
        .line_height(px(text_metrics::line_height(text_metrics::META)))
        .child(name.clone());
    control(
        scene,
        role,
        name,
        ancestors,
        row.on_click(scene.on_click_intent(Intent::Pick(pick))),
    )
}

/// The project `CommandItem` height: `px-2 py-1.5 typography-meta` (`command.tsx:121-131`)
/// around `ProjectLabel`'s `inline-flex` (`DraftTargetSelectors.tsx:131-150`).
pub(super) fn project_row_height(metrics: text_metrics::UiMetrics) -> f32 {
    let label_box = text_metrics::line_height(text_metrics::META).max(PROJECT_ICON);
    let line_box = text_metrics::inline_flex_line_box(metrics, label_box, text_metrics::META);
    2. * ROW_PADDING_Y + line_box
}

/// The popup panel's frame, one region.
pub(super) fn frame(scene: Scene<'_>, cx: &App, width: Option<f32>, content: Div) -> Div {
    let panel = div()
        .flex()
        .flex_col()
        .rounded(px(PANEL_RADIUS))
        .bg(cx.theme().popover)
        .overflow_hidden()
        .child(content);
    let outer = div().flex().flex_col();
    let sized = match width {
        Some(width) => outer.w(px(width)),
        None => outer,
    };
    region(scene.registry, Area::Popover, sized.child(panel))
}

#[cfg(test)]
#[path = "common_test.rs"]
mod common_test;
