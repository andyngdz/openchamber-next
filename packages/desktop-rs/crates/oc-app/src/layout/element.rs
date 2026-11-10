//! Elements that report their bounds to the [`LayoutRegistry`] while the frame is prepainted.

use gpui_kit::base::ElementExt;
use gpui_kit::{Div, InteractiveElement, IntoElement, ParentElement, Styled, canvas, div};

use super::registry::{Area, ControlSpec, LayoutRegistry, Reveal};

/// Wraps the app's root element, whose bounds are the content area.
pub(crate) fn content(registry: &LayoutRegistry, root: Div) -> Div {
    let registry = registry.clone();
    root.relative()
        .on_prepaint(move |bounds, _, _| registry.record_content(bounds))
}

/// Wraps a region's root so its bounds land in the registry. The probe measures the padding
/// box, so a region root carries no border of its own.
pub(crate) fn region(registry: &LayoutRegistry, area: Area, root: Div) -> Div {
    let registry = registry.clone();
    root.relative()
        .on_prepaint(move |bounds, _, _| registry.record_region(area, bounds))
}

/// Wraps one control. The wrapper is as large as `content`; its bounds probe is the first
/// child, so a control containing other controls is recorded before them, in DOM order.
/// A control the state hides stays laid out and is only made transparent.
pub(crate) fn control(
    registry: &LayoutRegistry,
    spec: ControlSpec,
    content: impl IntoElement,
) -> Div {
    let registry = registry.clone();
    let reveal = spec.reveal.clone();
    let probe = canvas(
        move |bounds, _, _| registry.record_control(spec, bounds),
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let wrapper = div()
        .relative()
        .flex()
        .opacity(resting_opacity(&reveal))
        .child(probe)
        .child(content);
    match reveal {
        Reveal::OnHover(group) => wrapper.group_hover(group, |style| style.opacity(1.)),
        Reveal::Always | Reveal::Hidden => wrapper,
    }
}

/// Opacity of a control while the pointer is elsewhere.
fn resting_opacity(reveal: &Reveal) -> f32 {
    match reveal {
        Reveal::Always => 1.,
        Reveal::OnHover(_) | Reveal::Hidden => 0.,
    }
}

#[cfg(test)]
#[path = "element_test.rs"]
mod element_test;
