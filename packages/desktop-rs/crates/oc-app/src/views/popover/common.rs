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
    role: Role,
    name: impl Into<SharedString>,
    ancestors: &[&str],
    pick: Pick,
) -> Div {
    let name: SharedString = name.into();
    let row = div()
        .id(name.clone())
        .h(px(34.2))
        .px(px(8.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .text_sm()
        .child(name.clone());
    control(
        scene,
        role,
        name,
        ancestors,
        row.on_click(scene.on_click_intent(Intent::Pick(pick))),
    )
}

/// The popup panel's frame, one region.
pub(super) fn frame(scene: Scene<'_>, cx: &App, width: Option<f32>, content: Div) -> Div {
    let panel = div()
        .flex()
        .flex_col()
        .rounded(px(12.))
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
