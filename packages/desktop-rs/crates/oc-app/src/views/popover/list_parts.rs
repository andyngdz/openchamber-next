//! The picker section header, its marks and the search field wrapper.

use super::list::{GAP, Header, ancestors};
use crate::icons::RemixIcon;
use crate::layout::{
    Area, ControlSpec, DrawSlot, LayoutRegistry, Role, control as control_element, placed,
};
use crate::views::text_metrics;
use gpui_kit::{
    AnyElement, Div, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, canvas,
    div, px,
};

/// The provider mark on a section header, `h-4 w-4` (`ModelPickerList.tsx:1104`).
const HEADER_ICON: f32 = 16.;
/// The check after the current model, `h-4 w-4` (`ModelPickerList.tsx:1063`).
const CHECK_ICON: f32 = 16.;
/// `font-semibold` on a section label (`ModelPickerList.tsx:942`).
const HEADER_WEIGHT: f32 = 600.;

/// A section header, recorded under its group ancestors.
pub(super) fn header(registry: &LayoutRegistry, header: &Header) -> Div {
    let ancestors = ancestors(header.ancestors.iter().map(String::as_str));
    let spec =
        ControlSpec::named(Area::Popover, Role::Button, header.name.clone()).under(&ancestors);
    let palette = crate::theme::palette();
    let muted_color: Hsla = palette.muted_foreground.into();
    // The Favorites star is `text-primary` (`ModelPickerList.tsx`), every other mark muted.
    let icon_color: Hsla = match header.icon {
        Some(RemixIcon::StarFill) => palette.primary.into(),
        _ => muted_color,
    };
    let icon_slot = record_header_mark(registry, &spec.key, header, icon_color);
    let label_slot = registry.record_text(
        &spec.key,
        crate::layout::draw::TextDraw::new(header.label.clone(), muted_color),
    );
    let line = text_metrics::line_height(text_metrics::MICRO);
    let content =
        div()
            .h(px(line))
            .flex()
            .items_center()
            .gap(px(GAP))
            .text_size(px(text_metrics::MICRO))
            .line_height(px(line))
            .font_weight(FontWeight(HEADER_WEIGHT))
            .text_color(muted_color)
            .children(header.icon.zip(icon_slot).map(|(icon, slot)| {
                placed(registry, slot, icon_box(icon, HEADER_ICON, icon_color))
            }))
            .children(header.logo.as_ref().map(|id| logo_box(id, HEADER_ICON)))
            .child(placed(
                registry,
                label_slot,
                div().child(header.label.clone()),
            ));
    control_element(registry, spec, content)
        .px(px(8.))
        .py(px(6.))
        .w_full()
}

/// Records a header's mark, a provider logo or a Remix icon, after dropping the draws of an
/// earlier render of the same header in this frame.
fn record_header_mark(
    registry: &LayoutRegistry,
    control: &SharedString,
    header: &Header,
    icon_color: Hsla,
) -> Option<DrawSlot> {
    registry.begin_draws(control);
    match (&header.logo, header.icon) {
        (Some(logo), _) => {
            record_logo(registry, control, logo);
            None
        }
        (None, Some(icon)) => {
            Some(registry.record_icon(control, crate::layout::draw::remix(icon, icon_color)))
        }
        (None, None) => None,
    }
}

/// The list with the search field, recorded under the reference's search key.
pub(super) fn search(
    registry: &LayoutRegistry,
    role: Role,
    placeholder: &str,
    list: impl IntoElement,
) -> Div {
    let spec = ControlSpec::named(Area::Popover, role, placeholder);
    control_element(registry, spec, list).w_full()
}

/// One icon box of a row or header, at `size`.
pub(super) fn icon_box(icon: RemixIcon, size: f32, color: Hsla) -> Div {
    div()
        .flex_shrink_0()
        .w(px(size))
        .h(px(size))
        .text_size(px(size))
        .text_color(color)
        .child(icon)
}

/// Records a provider logo under the id the capture names it by, `img:<file stem>` for a
/// bundled logo or the models.dev id; a provider showing the fallback icon records none.
pub(super) fn record_logo(registry: &LayoutRegistry, control: &SharedString, provider_id: &str) {
    let logo = crate::icons::provider::provider_logo(provider_id);
    if let Some(id) = logo.image_id() {
        registry.record_icon(control, crate::layout::draw::image(id.to_owned()));
    }
}

/// A provider logo box, drawn from the bundled set when the provider has one.
pub(super) fn logo_box(id: &str, size: f32) -> AnyElement {
    crate::views::provider_mark::provider_mark(size, &crate::icons::provider::provider_logo(id))
}

/// A row's context label, in the muted foreground.
pub(super) fn muted(text: &str, color: Hsla) -> Div {
    div()
        .flex_shrink_0()
        .text_size(px(text_metrics::MICRO))
        .line_height(px(text_metrics::line_height(text_metrics::MICRO)))
        .text_color(color)
        .child(text.to_owned())
}

/// The check after the current model.
pub(super) fn check_icon(color: Hsla) -> Div {
    div()
        .flex_shrink_0()
        .w(px(CHECK_ICON))
        .h(px(CHECK_ICON))
        .text_size(px(CHECK_ICON))
        .text_color(color)
        .child(RemixIcon::Check)
}

/// The invisible element that records a control's bounds in the frame.
pub(super) fn probe(registry: &LayoutRegistry, spec: ControlSpec) -> impl IntoElement {
    let registry = registry.clone();
    canvas(
        move |bounds, _, _| registry.record_control(spec.clone(), bounds),
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

#[cfg(test)]
#[path = "list_parts_test.rs"]
mod list_parts_test;
